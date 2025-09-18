use std::{
    collections::{HashMap, HashSet},
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use crate::ui::theme;
use chrono::NaiveDateTime;
use eframe::egui::{self, Widget};
use ps2_filetypes::{sjis, templates, IconSys, PSUEntryKind, TitleCfg, PSU};
use psu_packer::{ColorConfig, ColorFConfig, IconSysConfig, VectorConfig};
use tempfile::{tempdir, TempDir};

pub(crate) mod sas_timestamps;
pub mod ui;

use sas_timestamps::TimestampRules;

pub use ui::{dialogs, file_picker, pack_controls};

pub(crate) const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
pub(crate) const ICON_SYS_FLAG_OPTIONS: &[(u16, &str)] =
    &[(0, "Save Data"), (1, "System Software"), (4, "Settings")];
pub(crate) const ICON_SYS_TITLE_CHAR_LIMIT: usize = 16;
const ICON_SYS_UNSUPPORTED_CHAR_PLACEHOLDER: char = '\u{FFFD}';
const TIMESTAMP_RULES_FILE: &str = "timestamp_rules.json";
pub(crate) const REQUIRED_PROJECT_FILES: &[&str] =
    &["icon.icn", "icon.sys", "psu.toml", "title.cfg"];
const CENTERED_COLUMN_MAX_WIDTH: f32 = 1180.0;
const PACK_CONTROLS_TWO_COLUMN_MIN_WIDTH: f32 = 940.0;
const TITLE_CFG_GRID_SPACING: [f32; 2] = [28.0, 12.0];
const TITLE_CFG_SECTION_GAP: f32 = 20.0;
const TITLE_CFG_SECTION_HEADING_GAP: f32 = 6.0;
const TITLE_CFG_MULTILINE_ROWS: usize = 6;
const TITLE_CFG_SECTIONS: &[(&str, &[&str])] = &[
    (
        "Application identity",
        &["title", "Title", "Version", "Release", "Developer", "Genre"],
    ),
    (
        "Boot configuration",
        &["boot", "CfgVersion", "$ConfigSource", "source"],
    ),
    ("Description", &["Description", "Notes"]),
    (
        "Presentation",
        &[
            "Parental",
            "ParentalText",
            "Vmode",
            "VmodeText",
            "Aspect",
            "AspectText",
            "Scan",
            "ScanText",
        ],
    ),
    (
        "Players and devices",
        &["Players", "PlayersText", "Device", "DeviceText"],
    ),
    ("Ratings", &["Rating", "RatingText"]),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MissingFileReason {
    AlwaysRequired,
    ExplicitlyIncluded,
    TimestampAutomation,
}

impl MissingFileReason {
    fn detail(&self) -> Option<&'static str> {
        match self {
            MissingFileReason::AlwaysRequired => None,
            MissingFileReason::ExplicitlyIncluded => Some("listed in Include files"),
            MissingFileReason::TimestampAutomation => Some("needed for SAS timestamp automation"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MissingRequiredFile {
    pub(crate) name: String,
    pub(crate) reason: MissingFileReason,
}

impl MissingRequiredFile {
    fn always(name: &str) -> Self {
        Self {
            name: name.to_string(),
            reason: MissingFileReason::AlwaysRequired,
        }
    }

    fn included(name: &str) -> Self {
        Self {
            name: name.to_string(),
            reason: MissingFileReason::ExplicitlyIncluded,
        }
    }

    fn timestamp_rules() -> Self {
        Self {
            name: TIMESTAMP_RULES_FILE.to_string(),
            reason: MissingFileReason::TimestampAutomation,
        }
    }
}

fn split_icon_sys_title(title: &str, break_index: usize) -> (String, String) {
    let sanitized_chars: Vec<char> = title
        .chars()
        .map(|c| {
            if c.is_control() {
                ICON_SYS_UNSUPPORTED_CHAR_PLACEHOLDER
            } else {
                c
            }
        })
        .collect();

    let mut remaining_bytes = break_index;
    let mut break_in_chars = 0usize;
    if remaining_bytes > 0 {
        for ch in title.chars() {
            let mut utf8 = [0u8; 4];
            let encoded_len = sjis::encode_sjis(ch.encode_utf8(&mut utf8))
                .map(|bytes| bytes.len())
                .unwrap_or(1)
                .max(1);

            if remaining_bytes < encoded_len {
                break;
            }
            remaining_bytes -= encoded_len;
            break_in_chars += 1;
            if remaining_bytes == 0 {
                break;
            }
        }
    }

    let break_index = break_in_chars.min(sanitized_chars.len());
    let line1_count = break_index.min(ICON_SYS_TITLE_CHAR_LIMIT);
    let skip_count = line1_count;

    let line1: String = sanitized_chars.iter().take(line1_count).copied().collect();
    let line2: String = sanitized_chars
        .iter()
        .skip(skip_count)
        .take(ICON_SYS_TITLE_CHAR_LIMIT)
        .copied()
        .collect();

    (line1, line2)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SasPrefix {
    None,
    App,
    Apps,
    Ps1,
    Emu,
    Gme,
    Dst,
    Dbg,
    Raa,
    Rte,
    Sys,
    Zzy,
    Zzz,
}

pub(crate) const SAS_PREFIXES: [SasPrefix; 12] = [
    SasPrefix::App,
    SasPrefix::Apps,
    SasPrefix::Ps1,
    SasPrefix::Emu,
    SasPrefix::Gme,
    SasPrefix::Dst,
    SasPrefix::Dbg,
    SasPrefix::Raa,
    SasPrefix::Rte,
    SasPrefix::Sys,
    SasPrefix::Zzy,
    SasPrefix::Zzz,
];

impl Default for SasPrefix {
    fn default() -> Self {
        SasPrefix::App
    }
}

impl SasPrefix {
    pub const fn as_str(self) -> &'static str {
        match self {
            SasPrefix::None => "",
            SasPrefix::App => "APP_",
            SasPrefix::Apps => "APPS",
            SasPrefix::Ps1 => "PS1_",
            SasPrefix::Emu => "EMU_",
            SasPrefix::Gme => "GME_",
            SasPrefix::Dst => "DST_",
            SasPrefix::Dbg => "DBG_",
            SasPrefix::Raa => "RAA_",
            SasPrefix::Rte => "RTE_",
            SasPrefix::Sys => "SYS_",
            SasPrefix::Zzy => "ZZY_",
            SasPrefix::Zzz => "ZZZ_",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            SasPrefix::None => "(none)",
            _ => self.as_str(),
        }
    }

    pub(crate) fn iter_all() -> impl Iterator<Item = SasPrefix> {
        std::iter::once(SasPrefix::None).chain(SAS_PREFIXES.iter().copied())
    }

    pub(crate) fn split_from_name(name: &str) -> (SasPrefix, &str) {
        for prefix in SAS_PREFIXES {
            let value = prefix.as_str();
            if name.starts_with(value) {
                let remainder = &name[value.len()..];
                return (prefix, remainder);
            }
        }
        (SasPrefix::None, name)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum IconFlagSelection {
    Preset(usize),
    Custom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TimestampStrategy {
    None,
    InheritSource,
    SasRules,
    Manual,
}

impl Default for TimestampStrategy {
    fn default() -> Self {
        TimestampStrategy::None
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditorTab {
    PsuSettings,
    #[cfg(feature = "psu-toml-editor")]
    /// Enable the psu.toml editor again with `--features psu-toml-editor`.
    PsuToml,
    TitleCfg,
    IconSys,
    TimestampAuto,
}

#[derive(Default)]
struct TextFileEditor {
    content: String,
    modified: bool,
    load_error: Option<String>,
}

impl TextFileEditor {
    fn set_content(&mut self, content: String) {
        self.content = content;
        self.modified = false;
        self.load_error = None;
    }

    fn set_error_message(&mut self, message: String) {
        self.content.clear();
        self.modified = false;
        self.load_error = Some(message);
    }

    fn clear(&mut self) {
        self.content.clear();
        self.modified = false;
        self.load_error = None;
    }
}

struct PackJob {
    progress: Arc<Mutex<PackProgress>>,
    handle: Option<thread::JoinHandle<()>>,
}

enum PackProgress {
    InProgress,
    Finished(PackOutcome),
}

struct PackPreparation {
    folder: PathBuf,
    config: psu_packer::Config,
    missing_required_files: Vec<MissingRequiredFile>,
}

enum PackOutcome {
    Success {
        output_path: PathBuf,
    },
    Error {
        folder: PathBuf,
        output_path: PathBuf,
        error: psu_packer::Error,
    },
}

enum PendingPackAction {
    Pack {
        folder: PathBuf,
        output_path: PathBuf,
        config: psu_packer::Config,
        missing_required_files: Vec<MissingRequiredFile>,
    },
}

impl PendingPackAction {
    fn missing_files(&self) -> &[MissingRequiredFile] {
        match self {
            PendingPackAction::Pack {
                missing_required_files,
                ..
            } => missing_required_files,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct TimestampRulesUiState;

impl TimestampRulesUiState {
    pub(crate) fn from_rules(_: &TimestampRules) -> Self {
        Self
    }

    pub(crate) fn ensure_matches(&mut self, _: &TimestampRules) {}

    fn swap(&mut self, _: usize, _: usize) {}
}

pub struct PackerApp {
    pub(crate) folder: Option<PathBuf>,
    pub(crate) output: String,
    pub(crate) status: String,
    pub(crate) error_message: Option<String>,
    pub(crate) selected_prefix: SasPrefix,
    pub(crate) folder_base_name: String,
    pub(crate) psu_file_base_name: String,
    pub(crate) timestamp: Option<NaiveDateTime>,
    pub(crate) timestamp_strategy: TimestampStrategy,
    pub(crate) timestamp_from_rules: bool,
    pub(crate) source_timestamp: Option<NaiveDateTime>,
    pub(crate) manual_timestamp: Option<NaiveDateTime>,
    pub(crate) timestamp_rules: TimestampRules,
    pub(crate) timestamp_rules_loaded_from_file: bool,
    pub(crate) timestamp_rules_modified: bool,
    pub(crate) timestamp_rules_error: Option<String>,
    pub(crate) timestamp_rules_ui: TimestampRulesUiState,
    pub(crate) include_files: Vec<String>,
    pub(crate) exclude_files: Vec<String>,
    pub(crate) include_manual_entry: String,
    pub(crate) exclude_manual_entry: String,
    pub(crate) selected_include: Option<usize>,
    pub(crate) selected_exclude: Option<usize>,
    pub(crate) missing_required_project_files: Vec<MissingRequiredFile>,
    pending_pack_action: Option<PendingPackAction>,
    pub(crate) loaded_psu_path: Option<PathBuf>,
    pub(crate) loaded_psu_files: Vec<String>,
    pub(crate) show_exit_confirm: bool,
    pub(crate) exit_confirmed: bool,
    pub(crate) source_present_last_frame: bool,
    pub(crate) icon_sys_enabled: bool,
    pub(crate) icon_sys_title_line1: String,
    pub(crate) icon_sys_title_line2: String,
    pub(crate) icon_sys_flag_selection: IconFlagSelection,
    pub(crate) icon_sys_custom_flag: u16,
    pub(crate) icon_sys_background_transparency: u32,
    pub(crate) icon_sys_background_colors: [ColorConfig; 4],
    pub(crate) icon_sys_light_directions: [VectorConfig; 3],
    pub(crate) icon_sys_light_colors: [ColorFConfig; 3],
    pub(crate) icon_sys_ambient_color: ColorFConfig,
    pub(crate) icon_sys_selected_preset: Option<String>,
    pub(crate) icon_sys_use_existing: bool,
    pub(crate) icon_sys_existing: Option<IconSys>,
    zoom_factor: f32,
    pack_job: Option<PackJob>,
    temp_workspace: Option<TempDir>,
    editor_tab: EditorTab,
    psu_toml_editor: TextFileEditor,
    title_cfg_editor: TextFileEditor,
    psu_toml_sync_blocked: bool,
    theme: theme::Palette,
    #[cfg(test)]
    test_pack_job_started: bool,
}

struct ErrorMessage {
    message: String,
    failed_files: Vec<String>,
}

impl From<String> for ErrorMessage {
    fn from(message: String) -> Self {
        Self {
            message,
            failed_files: Vec::new(),
        }
    }
}

impl From<&str> for ErrorMessage {
    fn from(message: &str) -> Self {
        Self {
            message: message.to_owned(),
            failed_files: Vec::new(),
        }
    }
}

impl<S> From<(S, Vec<String>)> for ErrorMessage
where
    S: Into<String>,
{
    fn from((message, failed_files): (S, Vec<String>)) -> Self {
        Self {
            message: message.into(),
            failed_files,
        }
    }
}

impl Default for PackerApp {
    fn default() -> Self {
        let timestamp_rules = TimestampRules::default();
        let timestamp_rules_ui = TimestampRulesUiState::from_rules(&timestamp_rules);
        Self {
            folder: None,
            output: String::new(),
            status: String::new(),
            error_message: None,
            selected_prefix: SasPrefix::default(),
            folder_base_name: String::new(),
            psu_file_base_name: String::new(),
            timestamp: None,
            timestamp_strategy: TimestampStrategy::default(),
            timestamp_from_rules: false,
            source_timestamp: None,
            manual_timestamp: None,
            timestamp_rules,
            timestamp_rules_loaded_from_file: false,
            timestamp_rules_modified: false,
            timestamp_rules_error: None,
            timestamp_rules_ui,
            include_files: Vec::new(),
            exclude_files: Vec::new(),
            include_manual_entry: String::new(),
            exclude_manual_entry: String::new(),
            selected_include: None,
            selected_exclude: None,
            missing_required_project_files: Vec::new(),
            pending_pack_action: None,
            loaded_psu_path: None,
            loaded_psu_files: Vec::new(),
            show_exit_confirm: false,
            exit_confirmed: false,
            source_present_last_frame: false,
            icon_sys_enabled: false,
            icon_sys_title_line1: String::new(),
            icon_sys_title_line2: String::new(),
            icon_sys_flag_selection: IconFlagSelection::Preset(0),
            icon_sys_custom_flag: ICON_SYS_FLAG_OPTIONS[0].0,
            icon_sys_background_transparency: IconSysConfig::default_background_transparency(),
            icon_sys_background_colors: IconSysConfig::default_background_colors(),
            icon_sys_light_directions: IconSysConfig::default_light_directions(),
            icon_sys_light_colors: IconSysConfig::default_light_colors(),
            icon_sys_ambient_color: IconSysConfig::default_ambient_color(),
            icon_sys_selected_preset: None,
            icon_sys_use_existing: false,
            icon_sys_existing: None,
            zoom_factor: 1.0,
            pack_job: None,
            temp_workspace: None,
            editor_tab: EditorTab::PsuSettings,
            psu_toml_editor: TextFileEditor::default(),
            title_cfg_editor: TextFileEditor::default(),
            psu_toml_sync_blocked: false,
            theme: theme::Palette::default(),
            #[cfg(test)]
            test_pack_job_started: false,
        }
    }
}

impl PackerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        app.zoom_factor = cc.egui_ctx.pixels_per_point();
        theme::install(&cc.egui_ctx, &app.theme);
        app
    }

    fn timestamp_rules_path_from(folder: &Path) -> PathBuf {
        folder.join(TIMESTAMP_RULES_FILE)
    }

    pub(crate) fn timestamp_rules_path(&self) -> Option<PathBuf> {
        self.folder
            .as_ref()
            .map(|folder| Self::timestamp_rules_path_from(folder))
    }

    fn missing_required_project_files_for(&self, folder: &Path) -> Vec<MissingRequiredFile> {
        let mut missing = REQUIRED_PROJECT_FILES
            .iter()
            .filter_map(|name| {
                let candidate = folder.join(name);
                if candidate.is_file() {
                    None
                } else {
                    Some(MissingRequiredFile::always(name))
                }
            })
            .collect::<Vec<_>>();

        if self.include_requires_file("BOOT.ELF") {
            let candidate = folder.join("BOOT.ELF");
            if !candidate.is_file() {
                missing.push(MissingRequiredFile::included("BOOT.ELF"));
            }
        }

        if self.uses_timestamp_rules_file() {
            let candidate = folder.join(TIMESTAMP_RULES_FILE);
            if !candidate.is_file() {
                missing.push(MissingRequiredFile::timestamp_rules());
            }
        }

        missing
    }

    fn include_requires_file(&self, file_name: &str) -> bool {
        self.include_files
            .iter()
            .any(|entry| entry.eq_ignore_ascii_case(file_name))
    }

    fn uses_timestamp_rules_file(&self) -> bool {
        matches!(self.timestamp_strategy, TimestampStrategy::SasRules)
            && (self.timestamp_rules_loaded_from_file || self.timestamp_rules_modified)
    }

    pub(crate) fn refresh_missing_required_project_files(&mut self) {
        if let Some(folder) = self.folder.clone() {
            self.missing_required_project_files = self.missing_required_project_files_for(&folder);
        } else {
            self.missing_required_project_files.clear();
        }
    }

    pub(crate) fn pending_pack_missing_files(&self) -> Option<&[MissingRequiredFile]> {
        self.pending_pack_action
            .as_ref()
            .map(|action| action.missing_files())
    }

    pub(crate) fn confirm_pending_pack_action(&mut self) {
        if let Some(action) = self.pending_pack_action.take() {
            match action {
                PendingPackAction::Pack {
                    folder,
                    output_path,
                    config,
                    ..
                } => {
                    self.begin_pack_job(folder, output_path, config);
                }
            }
        }
    }

    pub(crate) fn cancel_pending_pack_action(&mut self) {
        self.pending_pack_action = None;
    }

    fn editor_tab_button(
        &mut self,
        ui: &mut egui::Ui,
        tab: EditorTab,
        label: &str,
        alert: bool,
        font: &egui::FontId,
    ) {
        let widget = EditorTabWidget::new(
            label,
            font.clone(),
            &self.theme,
            self.editor_tab == tab,
            alert,
        );
        let response = ui.add(widget);
        if response.clicked() {
            self.editor_tab = tab;
        }
    }

    pub(crate) fn load_timestamp_rules_from_folder(&mut self, folder: &Path) {
        let path = Self::timestamp_rules_path_from(folder);
        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<TimestampRules>(&content) {
                Ok(mut rules) => {
                    rules.sanitize();
                    self.timestamp_rules = rules;
                    self.timestamp_rules_error = None;
                    self.timestamp_rules_loaded_from_file = true;
                }
                Err(err) => {
                    self.timestamp_rules = TimestampRules::default();
                    self.timestamp_rules_error =
                        Some(format!("Failed to parse {}: {err}", path.display()));
                    self.timestamp_rules_loaded_from_file = true;
                }
            },
            Err(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    self.timestamp_rules = TimestampRules::default();
                    self.timestamp_rules_error = None;
                    self.timestamp_rules_loaded_from_file = false;
                } else {
                    self.timestamp_rules = TimestampRules::default();
                    self.timestamp_rules_error =
                        Some(format!("Failed to read {}: {err}", path.display()));
                    self.timestamp_rules_loaded_from_file = true;
                }
            }
        }

        self.timestamp_rules_ui = TimestampRulesUiState::from_rules(&self.timestamp_rules);
        self.timestamp_rules_modified = false;
    }

    pub(crate) fn save_timestamp_rules(&mut self) -> Result<PathBuf, String> {
        let Some(folder) = self.folder.as_ref() else {
            return Err("Select a folder before saving timestamp rules.".to_string());
        };

        self.timestamp_rules.sanitize();
        let serialized = serde_json::to_string_pretty(&self.timestamp_rules)
            .map_err(|err| format!("Failed to serialize timestamp rules: {err}"))?;

        let path = Self::timestamp_rules_path_from(folder);
        fs::write(&path, serialized)
            .map_err(|err| format!("Failed to write {}: {err}", path.display()))?;

        self.timestamp_rules_ui = TimestampRulesUiState::from_rules(&self.timestamp_rules);
        self.timestamp_rules_modified = false;
        self.timestamp_rules_error = None;
        self.timestamp_rules_loaded_from_file = true;
        Ok(path)
    }

    pub(crate) fn set_timestamp_strategy(&mut self, strategy: TimestampStrategy) {
        if self.timestamp_strategy == strategy {
            return;
        }

        self.timestamp_strategy = strategy;

        if matches!(self.timestamp_strategy, TimestampStrategy::Manual)
            && self.manual_timestamp.is_none()
        {
            if let Some(source) = self.source_timestamp {
                self.manual_timestamp = Some(source);
            } else if let Some(planned) = self.planned_timestamp_for_current_source() {
                self.manual_timestamp = Some(planned);
            }
        }

        self.refresh_timestamp_from_strategy();
    }

    pub(crate) fn refresh_timestamp_from_strategy(&mut self) {
        let new_timestamp = match self.timestamp_strategy {
            TimestampStrategy::None => None,
            TimestampStrategy::InheritSource => self.source_timestamp,
            TimestampStrategy::SasRules => self.planned_timestamp_for_current_source(),
            TimestampStrategy::Manual => self.manual_timestamp,
        };

        let changed = self.timestamp != new_timestamp;
        self.timestamp = new_timestamp;
        self.timestamp_from_rules = matches!(self.timestamp_strategy, TimestampStrategy::SasRules)
            && self.timestamp.is_some();

        if changed {
            self.refresh_psu_toml_editor();
        }
    }

    pub(crate) fn sync_timestamp_after_source_update(&mut self) {
        let planned = self.planned_timestamp_for_current_source();

        if matches!(self.timestamp_strategy, TimestampStrategy::None) {
            if self.source_timestamp.is_some() {
                self.timestamp_strategy = TimestampStrategy::InheritSource;
            } else if planned.is_some() {
                self.timestamp_strategy = TimestampStrategy::SasRules;
            }
        }

        if matches!(self.timestamp_strategy, TimestampStrategy::Manual)
            && self.manual_timestamp.is_none()
        {
            if let Some(source) = self.source_timestamp {
                self.manual_timestamp = Some(source);
            } else if let Some(planned) = planned {
                self.manual_timestamp = Some(planned);
            }
        }

        self.refresh_timestamp_from_strategy();
    }

    pub(crate) fn mark_timestamp_rules_modified(&mut self) {
        self.timestamp_rules_modified = true;
        self.recompute_timestamp_from_rules();
    }

    fn recompute_timestamp_from_rules(&mut self) {
        if !matches!(self.timestamp_strategy, TimestampStrategy::SasRules) {
            return;
        }

        self.refresh_timestamp_from_strategy();
    }

    pub(crate) fn apply_planned_timestamp(&mut self) {
        self.set_timestamp_strategy(TimestampStrategy::SasRules);
    }

    pub(crate) fn planned_timestamp_for_current_source(&self) -> Option<NaiveDateTime> {
        if let Some(folder) = self.folder.as_ref() {
            return sas_timestamps::planned_timestamp_for_folder(
                folder.as_path(),
                &self.timestamp_rules,
            );
        }

        let name = self.folder_name();
        if name.trim().is_empty() {
            return None;
        }

        sas_timestamps::planned_timestamp_for_name(&name, &self.timestamp_rules)
    }

    pub(crate) fn move_timestamp_category_up(&mut self, index: usize) {
        if index == 0 || index >= self.timestamp_rules.categories.len() {
            return;
        }
        self.timestamp_rules.categories.swap(index - 1, index);
        self.timestamp_rules_ui.swap(index - 1, index);
        self.mark_timestamp_rules_modified();
    }

    pub(crate) fn move_timestamp_category_down(&mut self, index: usize) {
        let len = self.timestamp_rules.categories.len();
        if index + 1 >= len {
            return;
        }
        self.timestamp_rules.categories.swap(index, index + 1);
        self.timestamp_rules_ui.swap(index, index + 1);
        self.mark_timestamp_rules_modified();
    }

    pub(crate) fn set_timestamp_aliases(&mut self, index: usize, aliases: Vec<String>) {
        if let Some(category) = self.timestamp_rules.categories.get_mut(index) {
            let allowed = sas_timestamps::canonical_aliases_for_category(&category.key);
            let selected: HashSet<&str> = aliases.iter().map(|alias| alias.as_str()).collect();
            let sanitized: Vec<String> = allowed
                .iter()
                .filter(|alias| selected.contains(**alias))
                .map(|alias| (*alias).to_string())
                .collect();

            if category.aliases != sanitized {
                category.aliases = sanitized;
                self.mark_timestamp_rules_modified();
            }
        }
    }

    pub(crate) fn reset_timestamp_rules_to_default(&mut self) {
        self.timestamp_rules = TimestampRules::default();
        self.timestamp_rules_error = None;
        self.timestamp_rules_ui = TimestampRulesUiState::from_rules(&self.timestamp_rules);
        self.timestamp_rules_loaded_from_file = false;
        self.mark_timestamp_rules_modified();
    }

    pub(crate) fn set_error_message<M>(&mut self, message: M)
    where
        M: Into<ErrorMessage>,
    {
        let message = message.into();
        let mut text = message.message;
        if !message.failed_files.is_empty() {
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str("Failed files: ");
            text.push_str(&message.failed_files.join(", "));
        }
        self.error_message = Some(text);
        self.status.clear();
    }

    pub(crate) fn format_missing_required_files_message(missing: &[MissingRequiredFile]) -> String {
        let formatted = missing
            .iter()
            .map(|entry| match entry.reason.detail() {
                Some(detail) => format!("• {} ({detail})", entry.name),
                None => format!("• {}", entry.name),
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "The selected folder is missing files needed to pack the PSU:\n{}",
            formatted
        )
    }

    pub(crate) fn clear_error_message(&mut self) {
        self.error_message = None;
    }

    pub(crate) fn reset_icon_sys_fields(&mut self) {
        self.icon_sys_enabled = false;
        self.icon_sys_use_existing = false;
        self.icon_sys_existing = None;
        self.icon_sys_title_line1.clear();
        self.icon_sys_title_line2.clear();
        self.icon_sys_flag_selection = IconFlagSelection::Preset(0);
        self.icon_sys_custom_flag = ICON_SYS_FLAG_OPTIONS[0].0;
        self.icon_sys_background_transparency = IconSysConfig::default_background_transparency();
        self.icon_sys_background_colors = IconSysConfig::default_background_colors();
        self.icon_sys_light_directions = IconSysConfig::default_light_directions();
        self.icon_sys_light_colors = IconSysConfig::default_light_colors();
        self.icon_sys_ambient_color = IconSysConfig::default_ambient_color();
        self.icon_sys_selected_preset = None;
    }

    pub(crate) fn apply_icon_sys_config(
        &mut self,
        icon_cfg: psu_packer::IconSysConfig,
        icon_sys_fallback: Option<&IconSys>,
    ) {
        let flag_value = icon_cfg.flags.value();
        self.icon_sys_enabled = true;
        self.icon_sys_use_existing = false;
        self.icon_sys_custom_flag = flag_value;
        if let Some(index) = ICON_SYS_FLAG_OPTIONS
            .iter()
            .position(|(value, _)| *value == flag_value)
        {
            self.icon_sys_flag_selection = IconFlagSelection::Preset(index);
        } else {
            self.icon_sys_flag_selection = IconFlagSelection::Custom;
        }

        let break_index = icon_cfg.linebreak_position() as usize;
        let (line1, line2) = split_icon_sys_title(&icon_cfg.title, break_index);
        self.icon_sys_title_line1 = line1;
        self.icon_sys_title_line2 = line2;

        self.icon_sys_background_transparency =
            icon_cfg.background_transparency.unwrap_or_else(|| {
                icon_sys_fallback
                    .map(|icon_sys| icon_sys.background_transparency)
                    .unwrap_or_else(IconSysConfig::default_background_transparency)
            });

        self.icon_sys_background_colors = if icon_cfg.background_colors.is_some() {
            icon_cfg.background_colors_array()
        } else if let Some(icon_sys) = icon_sys_fallback {
            let mut colors = IconSysConfig::default_background_colors();
            for (target, color) in colors.iter_mut().zip(icon_sys.background_colors.iter()) {
                *target = ColorConfig {
                    r: color.r,
                    g: color.g,
                    b: color.b,
                    a: color.a,
                };
            }
            colors
        } else {
            IconSysConfig::default_background_colors()
        };

        self.icon_sys_light_directions = if icon_cfg.light_directions.is_some() {
            icon_cfg.light_directions_array()
        } else if let Some(icon_sys) = icon_sys_fallback {
            let mut directions = IconSysConfig::default_light_directions();
            for (target, direction) in directions.iter_mut().zip(icon_sys.light_directions.iter()) {
                *target = VectorConfig {
                    x: direction.x,
                    y: direction.y,
                    z: direction.z,
                    w: direction.w,
                };
            }
            directions
        } else {
            IconSysConfig::default_light_directions()
        };

        self.icon_sys_light_colors = if icon_cfg.light_colors.is_some() {
            icon_cfg.light_colors_array()
        } else if let Some(icon_sys) = icon_sys_fallback {
            let mut colors = IconSysConfig::default_light_colors();
            for (target, color) in colors.iter_mut().zip(icon_sys.light_colors.iter()) {
                *target = ColorFConfig {
                    r: color.r,
                    g: color.g,
                    b: color.b,
                    a: color.a,
                };
            }
            colors
        } else {
            IconSysConfig::default_light_colors()
        };

        self.icon_sys_ambient_color = if let Some(color) = icon_cfg.ambient_color {
            color
        } else if let Some(icon_sys) = icon_sys_fallback {
            ColorFConfig {
                r: icon_sys.ambient_color.r,
                g: icon_sys.ambient_color.g,
                b: icon_sys.ambient_color.b,
                a: icon_sys.ambient_color.a,
            }
        } else {
            IconSysConfig::default_ambient_color()
        };
        self.icon_sys_selected_preset = icon_cfg.preset.clone();
    }

    pub(crate) fn apply_icon_sys_file(&mut self, icon_sys: &IconSys) {
        let flag_value = icon_sys.flags;
        self.icon_sys_enabled = true;
        self.icon_sys_use_existing = true;
        self.icon_sys_existing = Some(icon_sys.clone());
        self.icon_sys_custom_flag = flag_value;
        if let Some(index) = ICON_SYS_FLAG_OPTIONS
            .iter()
            .position(|(value, _)| *value == flag_value)
        {
            self.icon_sys_flag_selection = IconFlagSelection::Preset(index);
        } else {
            self.icon_sys_flag_selection = IconFlagSelection::Custom;
        }

        let break_index = icon_sys.linebreak_pos as usize;
        let (line1, line2) = split_icon_sys_title(&icon_sys.title, break_index);
        self.icon_sys_title_line1 = line1;
        self.icon_sys_title_line2 = line2;

        self.icon_sys_background_transparency = icon_sys.background_transparency;
        for (target, color) in self
            .icon_sys_background_colors
            .iter_mut()
            .zip(icon_sys.background_colors.iter())
        {
            *target = ColorConfig {
                r: color.r,
                g: color.g,
                b: color.b,
                a: color.a,
            };
        }

        for (target, direction) in self
            .icon_sys_light_directions
            .iter_mut()
            .zip(icon_sys.light_directions.iter())
        {
            *target = VectorConfig {
                x: direction.x,
                y: direction.y,
                z: direction.z,
                w: direction.w,
            };
        }

        for (target, color) in self
            .icon_sys_light_colors
            .iter_mut()
            .zip(icon_sys.light_colors.iter())
        {
            *target = ColorFConfig {
                r: color.r,
                g: color.g,
                b: color.b,
                a: color.a,
            };
        }

        self.icon_sys_ambient_color = ColorFConfig {
            r: icon_sys.ambient_color.r,
            g: icon_sys.ambient_color.g,
            b: icon_sys.ambient_color.b,
            a: icon_sys.ambient_color.a,
        };
        self.icon_sys_selected_preset = None;
    }

    pub(crate) fn clear_icon_sys_preset(&mut self) {
        self.icon_sys_selected_preset = None;
    }

    pub(crate) fn reset_metadata_fields(&mut self) {
        self.selected_prefix = SasPrefix::default();
        self.folder_base_name.clear();
        self.psu_file_base_name.clear();
        self.timestamp = None;
        self.timestamp_strategy = TimestampStrategy::None;
        self.timestamp_from_rules = false;
        self.source_timestamp = None;
        self.manual_timestamp = None;
        self.include_files.clear();
        self.exclude_files.clear();
        self.include_manual_entry.clear();
        self.exclude_manual_entry.clear();
        self.selected_include = None;
        self.selected_exclude = None;
        self.reset_icon_sys_fields();
    }

    pub(crate) fn folder_name(&self) -> String {
        let mut name = String::from(self.selected_prefix.as_str());
        name.push_str(&self.folder_base_name);
        name
    }

    fn effective_psu_file_base_name(&self) -> Option<String> {
        let trimmed_file = self.psu_file_base_name.trim();
        if !trimmed_file.is_empty() {
            return Some(trimmed_file.to_string());
        }

        let trimmed_folder = self.folder_base_name.trim();
        if trimmed_folder.is_empty() {
            None
        } else {
            Some(trimmed_folder.to_string())
        }
    }

    fn existing_output_directory(&self) -> Option<PathBuf> {
        let trimmed_output = self.output.trim();
        if trimmed_output.is_empty() {
            return None;
        }

        let path = Path::new(trimmed_output);
        path.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(|parent| parent.to_path_buf())
    }

    fn loaded_psu_directory(&self) -> Option<PathBuf> {
        self.loaded_psu_path
            .as_ref()
            .and_then(|path| path.parent())
            .map(|parent| parent.to_path_buf())
    }

    fn default_output_directory(&self, fallback_dir: Option<&Path>) -> Option<PathBuf> {
        if let Some(existing) = self.existing_output_directory() {
            return Some(existing);
        }

        if let Some(dir) = fallback_dir {
            return Some(dir.to_path_buf());
        }

        if let Some(folder) = self.folder.as_ref() {
            return Some(folder.clone());
        }

        self.loaded_psu_directory()
    }

    pub(crate) fn default_output_path(&self) -> Option<PathBuf> {
        self.default_output_path_with(None)
    }

    pub(crate) fn default_output_path_with(&self, fallback_dir: Option<&Path>) -> Option<PathBuf> {
        let file_name = self.default_output_file_name()?;
        let directory = self.default_output_directory(fallback_dir);
        Some(match directory {
            Some(dir) => dir.join(file_name),
            None => PathBuf::from(file_name),
        })
    }

    pub(crate) fn default_output_file_name(&self) -> Option<String> {
        let base_name = self.effective_psu_file_base_name()?;
        let mut stem = String::from(self.selected_prefix.as_str());
        stem.push_str(&base_name);
        if stem.is_empty() {
            None
        } else {
            Some(format!("{stem}.psu"))
        }
    }

    fn update_output_if_matches_default(&mut self, previous_default_output: Option<String>) {
        let should_update = if self.output.trim().is_empty() {
            true
        } else if let Some(previous_default) = previous_default_output {
            Path::new(&self.output)
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name == previous_default)
                .unwrap_or(false)
        } else {
            false
        };

        if should_update {
            match self.default_output_path() {
                Some(path) => {
                    self.output = path.display().to_string();
                }
                None => self.output.clear(),
            }
        }
    }

    pub(crate) fn metadata_inputs_changed(&mut self, previous_default_output: Option<String>) {
        if self.psu_file_base_name.trim().is_empty() {
            let trimmed_folder = self.folder_base_name.trim();
            if !trimmed_folder.is_empty() {
                self.psu_file_base_name = trimmed_folder.to_string();
            }
        }

        self.update_output_if_matches_default(previous_default_output);
        self.ensure_timestamp_strategy_default();
        if matches!(self.timestamp_strategy, TimestampStrategy::SasRules) {
            self.refresh_timestamp_from_strategy();
        }
        self.refresh_psu_toml_editor();
    }

    fn ensure_timestamp_strategy_default(&mut self) {
        if !matches!(self.timestamp_strategy, TimestampStrategy::None) {
            return;
        }

        let recommended = if self.source_timestamp.is_some() {
            Some(TimestampStrategy::InheritSource)
        } else if self.planned_timestamp_for_current_source().is_some() {
            Some(TimestampStrategy::SasRules)
        } else {
            Some(TimestampStrategy::Manual)
        };

        if let Some(strategy) = recommended {
            self.set_timestamp_strategy(strategy);
        }
    }

    pub(crate) fn set_folder_name_from_full(&mut self, name: &str) {
        let (prefix, remainder) = SasPrefix::split_from_name(name);
        self.selected_prefix = prefix;
        self.folder_base_name = remainder.to_string();
    }

    pub(crate) fn set_psu_file_base_from_full(&mut self, file_stem: &str) {
        let (prefix, remainder) = SasPrefix::split_from_name(file_stem);
        if prefix == SasPrefix::None || prefix == self.selected_prefix {
            self.psu_file_base_name = remainder.to_string();
        } else {
            self.psu_file_base_name = file_stem.to_string();
        }
    }

    pub(crate) fn icon_flag_label(&self) -> String {
        match self.icon_sys_flag_selection {
            IconFlagSelection::Preset(index) => ICON_SYS_FLAG_OPTIONS
                .get(index)
                .map(|(_, label)| (*label).to_string())
                .unwrap_or_else(|| format!("Preset {index}")),
            IconFlagSelection::Custom => {
                format!("Custom (0x{:04X})", self.icon_sys_custom_flag)
            }
        }
    }

    pub(crate) fn selected_icon_flag_value(&self) -> Result<u16, String> {
        match self.icon_sys_flag_selection {
            IconFlagSelection::Preset(index) => ICON_SYS_FLAG_OPTIONS
                .get(index)
                .map(|(value, _)| *value)
                .ok_or_else(|| "Invalid icon.sys flag selection".to_string()),
            IconFlagSelection::Custom => Ok(self.icon_sys_custom_flag),
        }
    }

    pub(crate) fn missing_include_files(&self, folder: &Path) -> Vec<String> {
        if self.include_files.is_empty() {
            return Vec::new();
        }

        self.include_files
            .iter()
            .filter_map(|file| {
                let candidate = folder.join(file);
                if candidate.is_file() {
                    None
                } else {
                    Some(file.clone())
                }
            })
            .collect()
    }

    pub(crate) fn handle_pack_request(&mut self) {
        if self.is_pack_running() {
            return;
        }

        let Some(preparation) = self.prepare_pack_inputs() else {
            return;
        };

        let output_path = PathBuf::from(&self.output);
        let PackPreparation {
            folder,
            config,
            missing_required_files,
        } = preparation;

        if missing_required_files.is_empty() {
            self.begin_pack_job(folder, output_path, config);
        } else {
            self.pending_pack_action = Some(PendingPackAction::Pack {
                folder,
                output_path,
                config,
                missing_required_files,
            });
        }
    }

    pub(crate) fn handle_update_psu_request(&mut self) {
        if self.is_pack_running() {
            return;
        }

        if self.loaded_psu_path.is_none() && self.output.trim().is_empty() {
            if !self.ensure_output_destination_selected() {
                return;
            }
        }

        let destination = match self.determine_update_destination() {
            Ok(path) => path,
            Err(message) => {
                self.set_error_message(message);
                return;
            }
        };

        if !destination.exists() {
            self.set_error_message(format!(
                "Cannot update because {} does not exist.",
                destination.display()
            ));
            return;
        }

        let mut temp_workspace_to_hold: Option<TempDir> = None;
        let preparation_result = if self.folder.is_some() {
            self.prepare_pack_inputs()
        } else if self.loaded_psu_path.is_some() {
            let (workspace, export_root) = match self.prepare_loaded_psu_workspace() {
                Ok(result) => result,
                Err(message) => {
                    self.set_error_message(message);
                    return;
                }
            };
            let preparation = self.prepare_pack_inputs_for_folder(export_root, None, true);
            if preparation.is_some() {
                temp_workspace_to_hold = Some(workspace);
            }
            preparation
        } else {
            self.prepare_pack_inputs()
        };

        let Some(preparation) = preparation_result else {
            return;
        };

        if !preparation.missing_required_files.is_empty() {
            self.pending_pack_action = None;
            self.temp_workspace = None;
            return;
        }

        let PackPreparation { folder, config, .. } = preparation;

        self.temp_workspace = temp_workspace_to_hold;
        self.begin_pack_job(folder, destination, config);
    }

    pub(crate) fn handle_save_as_folder_with_contents(&mut self) {
        if self.is_pack_running() {
            return;
        }

        if self.loaded_psu_path.is_none() && self.output.trim().is_empty() {
            if !self.ensure_output_destination_selected() {
                return;
            }
        }

        let source_path = match self.determine_export_source_path() {
            Ok(path) => path,
            Err(message) => {
                self.set_error_message(message);
                return;
            }
        };

        let Some(destination_parent) = rfd::FileDialog::new().pick_folder() else {
            return;
        };

        match self.export_psu_to_folder(&source_path, &destination_parent) {
            Ok(export_root) => {
                self.clear_error_message();
                self.status = format!(
                    "Exported PSU contents from {} to {}",
                    source_path.display(),
                    export_root.display()
                );
            }
            Err(message) => {
                self.set_error_message(message);
            }
        }
    }

    fn prepare_pack_inputs(&mut self) -> Option<PackPreparation> {
        let Some(folder) = self.folder.clone() else {
            self.set_error_message("Please select a folder");
            return None;
        };

        self.prepare_pack_inputs_for_folder(folder, None, false)
    }

    fn prepare_pack_inputs_for_folder(
        &mut self,
        folder: PathBuf,
        config_override: Option<psu_packer::Config>,
        allow_missing_psu_toml: bool,
    ) -> Option<PackPreparation> {
        if self.folder_base_name.trim().is_empty() {
            self.set_error_message("Please provide a folder name");
            return None;
        }

        if self.psu_file_base_name.trim().is_empty() {
            let trimmed_folder = self.folder_base_name.trim();
            if trimmed_folder.is_empty() {
                self.set_error_message("Please provide a PSU filename");
                return None;
            }
            self.psu_file_base_name = trimmed_folder.to_string();
        }

        if !self.ensure_output_destination_selected() {
            return None;
        }

        let mut missing = self.missing_required_project_files_for(&folder);
        if allow_missing_psu_toml {
            missing.retain(|entry| !entry.name.eq_ignore_ascii_case("psu.toml"));
        }
        self.missing_required_project_files = missing.clone();
        if !missing.is_empty() {
            let message = Self::format_missing_required_files_message(&missing);
            let failed_files = missing.iter().map(|entry| entry.name.clone()).collect();
            self.set_error_message((message, failed_files));
        }

        let config = match config_override {
            Some(config) => config,
            None => match self.build_config() {
                Ok(config) => config,
                Err(err) => {
                    self.set_error_message(err);
                    self.pending_pack_action = None;
                    return None;
                }
            },
        };

        Some(PackPreparation {
            folder,
            config,
            missing_required_files: missing,
        })
    }

    fn determine_update_destination(&self) -> Result<PathBuf, String> {
        if let Some(path) = &self.loaded_psu_path {
            return Ok(path.clone());
        }

        let trimmed = self.output.trim();
        if trimmed.is_empty() {
            Err("Load a PSU file or set the output path before updating.".to_string())
        } else {
            Ok(PathBuf::from(trimmed))
        }
    }

    fn determine_export_source_path(&self) -> Result<PathBuf, String> {
        if let Some(path) = &self.loaded_psu_path {
            return Ok(path.clone());
        }

        let trimmed = self.output.trim();
        if trimmed.is_empty() {
            Err("Load a PSU file or select a packed PSU before exporting its contents.".to_string())
        } else {
            Ok(PathBuf::from(trimmed))
        }
    }

    fn export_psu_to_folder(
        &self,
        source_path: &Path,
        destination_parent: &Path,
    ) -> Result<PathBuf, String> {
        if !source_path.is_file() {
            return Err(format!(
                "Cannot export because {} does not exist.",
                source_path.display()
            ));
        }

        let data = fs::read(source_path)
            .map_err(|err| format!("Failed to read {}: {err}", source_path.display()))?;

        let parsed = std::panic::catch_unwind(|| PSU::new(data))
            .map_err(|_| format!("Failed to parse PSU file {}", source_path.display()))?;

        let entries = parsed.entries();
        let root_name = entries
            .iter()
            .find(|entry| {
                matches!(entry.kind, PSUEntryKind::Directory)
                    && entry.name != "."
                    && entry.name != ".."
            })
            .map(|entry| entry.name.clone())
            .ok_or_else(|| format!("{} does not contain PSU metadata", source_path.display()))?;

        if root_name.trim().is_empty() {
            return Err(format!(
                "{} does not contain a valid root directory entry.",
                source_path.display()
            ));
        }

        let export_root = destination_parent.join(&root_name);
        fs::create_dir_all(&export_root)
            .map_err(|err| format!("Failed to create {}: {err}", export_root.display()))?;

        for entry in entries {
            match entry.kind {
                PSUEntryKind::Directory => {
                    if entry.name == "." || entry.name == ".." {
                        continue;
                    }

                    let target = if entry.name == root_name {
                        export_root.clone()
                    } else {
                        export_root.join(&entry.name)
                    };

                    fs::create_dir_all(&target)
                        .map_err(|err| format!("Failed to create {}: {err}", target.display()))?;
                }
                PSUEntryKind::File => {
                    let Some(contents) = entry.contents else {
                        return Err(format!(
                            "{} is missing file data in the PSU archive.",
                            entry.name
                        ));
                    };

                    let target = export_root.join(&entry.name);
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent).map_err(|err| {
                            format!("Failed to create {}: {err}", parent.display())
                        })?;
                    }

                    fs::write(&target, contents)
                        .map_err(|err| format!("Failed to write {}: {err}", target.display()))?;
                }
            }
        }

        Ok(export_root)
    }

    fn prepare_loaded_psu_workspace(&self) -> Result<(TempDir, PathBuf), String> {
        let source_path = self
            .loaded_psu_path
            .as_ref()
            .ok_or_else(|| "No PSU file is currently loaded.".to_string())?;
        let temp_dir =
            tempdir().map_err(|err| format!("Failed to create temporary workspace: {err}"))?;
        let export_root = self
            .export_psu_to_folder(source_path, temp_dir.path())
            .map_err(|err| format!("Failed to export loaded PSU: {err}"))?;
        Ok((temp_dir, export_root))
    }

    pub(crate) fn reload_project_files(&mut self) {
        if let Some(folder) = self.folder.clone() {
            load_text_file_into_editor(folder.as_path(), "psu.toml", &mut self.psu_toml_editor);
            load_text_file_into_editor(folder.as_path(), "title.cfg", &mut self.title_cfg_editor);
            self.psu_toml_sync_blocked = false;
            self.refresh_missing_required_project_files();
        } else {
            self.clear_text_editors();
            self.missing_required_project_files.clear();
        }
    }

    #[cfg(feature = "psu-toml-editor")]
    pub(crate) fn apply_psu_toml_edits(&mut self) -> bool {
        let temp_dir = match tempdir() {
            Ok(dir) => dir,
            Err(err) => {
                self.set_error_message(format!(
                    "Failed to prepare temporary psu.toml for parsing: {err}"
                ));
                return false;
            }
        };

        let config_path = temp_dir.path().join("psu.toml");
        if let Err(err) = fs::write(&config_path, self.psu_toml_editor.content.as_bytes()) {
            self.set_error_message(format!("Failed to write temporary psu.toml: {err}"));
            return false;
        }

        let config = match psu_packer::load_config(temp_dir.path()) {
            Ok(config) => config,
            Err(err) => {
                self.set_error_message(format!("Failed to parse psu.toml: {err}"));
                return false;
            }
        };

        let previous_default_output = self.default_output_file_name();

        let psu_packer::Config {
            name,
            timestamp,
            include,
            exclude,
            icon_sys,
        } = config;

        self.set_folder_name_from_full(&name);
        self.psu_file_base_name = self.folder_base_name.clone();
        self.source_timestamp = timestamp;
        self.manual_timestamp = timestamp;
        self.timestamp = timestamp;
        self.timestamp_strategy = if timestamp.is_some() {
            TimestampStrategy::Manual
        } else {
            TimestampStrategy::None
        };
        self.timestamp_from_rules = false;
        self.metadata_inputs_changed(previous_default_output);

        self.include_files = include.unwrap_or_default();
        self.exclude_files = exclude.unwrap_or_default();
        self.selected_include = None;
        self.selected_exclude = None;

        let existing_icon_sys = self.icon_sys_existing.clone();

        match icon_sys {
            Some(icon_cfg) => {
                self.apply_icon_sys_config(icon_cfg, existing_icon_sys.as_ref());
            }
            None => {
                if let Some(existing_icon_sys) = existing_icon_sys.as_ref() {
                    self.apply_icon_sys_file(existing_icon_sys);
                } else {
                    self.reset_icon_sys_fields();
                }
            }
        }

        self.psu_toml_sync_blocked = false;
        self.clear_error_message();
        self.status = "Applied psu.toml edits in memory.".to_string();
        true
    }

    pub(crate) fn apply_title_cfg_edits(&mut self) -> bool {
        let cfg = TitleCfg::new(self.title_cfg_editor.content.clone());
        if !cfg.has_mandatory_fields() {
            self.set_error_message(
                "title.cfg is missing mandatory fields. Please include the required keys.",
            );
            return false;
        }

        self.clear_error_message();
        self.status = "Validated title.cfg contents.".to_string();
        true
    }

    fn clear_text_editors(&mut self) {
        #[cfg(feature = "psu-toml-editor")]
        {
            self.psu_toml_editor.clear();
            self.psu_toml_sync_blocked = false;
        }
        self.title_cfg_editor.clear();
    }

    #[cfg(feature = "psu-toml-editor")]
    pub(crate) fn create_psu_toml_from_template(&mut self) {
        self.create_file_from_template(
            "psu.toml",
            templates::PSU_TOML_TEMPLATE,
            EditorTab::PsuToml,
        );
    }

    pub(crate) fn create_title_cfg_from_template(&mut self) {
        self.create_file_from_template(
            "title.cfg",
            templates::TITLE_CFG_TEMPLATE,
            EditorTab::TitleCfg,
        );
    }

    fn create_file_from_template(&mut self, file_name: &str, template: &str, tab: EditorTab) {
        if let Some(folder) = self.folder.clone() {
            let path = folder.join(file_name);
            if path.exists() {
                self.set_error_message(format!(
                    "{} already exists in the selected folder.",
                    path.display()
                ));
                return;
            }

            if let Err(err) = fs::write(&path, template) {
                self.set_error_message(format!("Failed to create {}: {}", path.display(), err));
                return;
            }

            self.status = format!("Created {} from template.", path.display());
            self.clear_error_message();
            self.reload_project_files();
        } else {
            if let Some(editor) = self.editor_for_text_tab(tab) {
                editor.set_content(template.to_string());
                editor.modified = true;
                self.clear_error_message();
                self.status = format!(
                    "Loaded default {file_name} template in the editor. Select a folder to save it."
                );
            } else {
                self.set_error_message(format!(
                    "Select a folder before creating {file_name} from the template."
                ));
                return;
            }
        }

        match tab {
            EditorTab::PsuSettings => self.open_psu_settings_tab(),
            #[cfg(feature = "psu-toml-editor")]
            EditorTab::PsuToml => self.open_psu_toml_tab(),
            EditorTab::TitleCfg => self.open_title_cfg_tab(),
            EditorTab::IconSys => self.open_icon_sys_tab(),
            EditorTab::TimestampAuto => self.open_timestamp_auto_tab(),
        }
    }

    #[cfg(feature = "psu-toml-editor")]
    fn editor_for_text_tab(&mut self, tab: EditorTab) -> Option<&mut TextFileEditor> {
        match tab {
            EditorTab::PsuToml => Some(&mut self.psu_toml_editor),
            EditorTab::TitleCfg => Some(&mut self.title_cfg_editor),
            _ => None,
        }
    }

    #[cfg(not(feature = "psu-toml-editor"))]
    fn editor_for_text_tab(&mut self, tab: EditorTab) -> Option<&mut TextFileEditor> {
        match tab {
            EditorTab::TitleCfg => Some(&mut self.title_cfg_editor),
            _ => None,
        }
    }

    pub(crate) fn open_psu_settings_tab(&mut self) {
        self.editor_tab = EditorTab::PsuSettings;
    }

    #[cfg(feature = "psu-toml-editor")]
    pub(crate) fn open_psu_toml_tab(&mut self) {
        self.editor_tab = EditorTab::PsuToml;
    }

    pub(crate) fn open_title_cfg_tab(&mut self) {
        self.editor_tab = EditorTab::TitleCfg;
    }

    pub(crate) fn open_icon_sys_tab(&mut self) {
        self.editor_tab = EditorTab::IconSys;
    }

    pub(crate) fn open_timestamp_auto_tab(&mut self) {
        self.editor_tab = EditorTab::TimestampAuto;
    }

    fn has_source(&self) -> bool {
        self.folder.is_some() || self.loaded_psu_path.is_some() || !self.loaded_psu_files.is_empty()
    }

    fn showing_loaded_psu(&self) -> bool {
        self.folder.is_none()
            && (self.loaded_psu_path.is_some() || !self.loaded_psu_files.is_empty())
    }

    pub(crate) fn is_pack_running(&self) -> bool {
        self.pack_job.is_some()
    }

    #[cfg(not(test))]
    fn begin_pack_job(
        &mut self,
        folder: PathBuf,
        output_path: PathBuf,
        config: psu_packer::Config,
    ) {
        self.pending_pack_action = None;
        self.start_pack_job(folder, output_path, config);
    }

    #[cfg(test)]
    fn begin_pack_job(
        &mut self,
        folder: PathBuf,
        output_path: PathBuf,
        config: psu_packer::Config,
    ) {
        self.pending_pack_action = None;
        self.test_pack_job_started = true;
        self.start_pack_job(folder, output_path, config);
    }

    pub(crate) fn start_pack_job(
        &mut self,
        folder: PathBuf,
        output_path: PathBuf,
        config: psu_packer::Config,
    ) {
        if self.pack_job.is_some() {
            return;
        }

        let progress = Arc::new(Mutex::new(PackProgress::InProgress));
        let thread_progress = Arc::clone(&progress);

        let handle = thread::spawn(move || {
            let result =
                psu_packer::pack_with_config(folder.as_path(), output_path.as_path(), config);

            let outcome = match result {
                Ok(_) => PackOutcome::Success {
                    output_path: output_path.clone(),
                },
                Err(error) => PackOutcome::Error {
                    folder: folder.clone(),
                    output_path: output_path.clone(),
                    error,
                },
            };

            let mut guard = thread_progress
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            *guard = PackProgress::Finished(outcome);
        });

        self.status = "Packing…".to_string();
        self.clear_error_message();
        self.pack_job = Some(PackJob {
            progress,
            handle: Some(handle),
        });
    }

    fn pack_progress_value(&self) -> Option<f32> {
        let job = self.pack_job.as_ref()?;
        let guard = job.progress.lock().ok()?;
        Some(match &*guard {
            PackProgress::InProgress => 0.0,
            PackProgress::Finished(_) => 1.0,
        })
    }

    fn poll_pack_job(&mut self) {
        let Some(mut job) = self.pack_job.take() else {
            return;
        };

        let outcome = match job.progress.lock() {
            Ok(mut guard) => {
                if let PackProgress::Finished(_) = &*guard {
                    if let PackProgress::Finished(outcome) =
                        std::mem::replace(&mut *guard, PackProgress::InProgress)
                    {
                        Some(outcome)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Err(poison) => {
                let mut guard = poison.into_inner();
                if let PackProgress::Finished(_) = &*guard {
                    if let PackProgress::Finished(outcome) =
                        std::mem::replace(&mut *guard, PackProgress::InProgress)
                    {
                        Some(outcome)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };

        if let Some(outcome) = outcome {
            if let Some(handle) = job.handle.take() {
                let _ = handle.join();
            }

            self.temp_workspace = None;

            match outcome {
                PackOutcome::Success { output_path } => {
                    self.status = format!("Packed to {}", output_path.display());
                    self.clear_error_message();
                }
                PackOutcome::Error {
                    folder,
                    output_path,
                    error,
                } => {
                    let message = self.format_pack_error(&folder, &output_path, error);
                    self.set_error_message(message);
                }
            }
        } else {
            self.pack_job = Some(job);
        }
    }
}

fn load_text_file_into_editor(folder: &Path, file_name: &str, editor: &mut TextFileEditor) {
    let path = folder.join(file_name);
    match fs::read_to_string(&path) {
        Ok(content) => {
            editor.set_content(content);
        }
        Err(err) => {
            if err.kind() == io::ErrorKind::NotFound {
                editor
                    .set_error_message(format!("{} not found in the selected folder.", file_name));
            } else {
                editor.set_error_message(format!("Failed to read {}: {err}", file_name));
            }
        }
    }
}

#[cfg(test)]
mod packer_app_tests {
    use super::*;
    use psu_packer::Config as PsuConfig;
    use std::{path::Path, thread, time::Duration};
    use tempfile::tempdir;

    fn wait_for_pack_completion(app: &mut PackerApp) {
        while app.pack_job.is_some() {
            thread::sleep(Duration::from_millis(10));
            app.poll_pack_job();
        }
    }

    fn write_required_files(folder: &Path) {
        for file in REQUIRED_PROJECT_FILES {
            let path = folder.join(file);
            fs::write(&path, b"data").expect("write required file");
        }
    }

    #[test]
    fn metadata_inputs_fill_missing_psu_filename() {
        let workspace = tempdir().expect("temp workspace");
        let project_dir = workspace.path().join("project");
        fs::create_dir_all(&project_dir).expect("create project folder");

        let mut app = PackerApp::default();
        app.folder = Some(project_dir.clone());
        app.folder_base_name = "SAVE".to_string();
        app.psu_file_base_name.clear();

        let previous_default = app.default_output_file_name();
        app.metadata_inputs_changed(previous_default);

        assert_eq!(app.psu_file_base_name, "SAVE");
        assert!(app.output.ends_with("APP_SAVE.psu"));
    }

    #[test]
    fn prepare_pack_inputs_sets_default_output_path() {
        let workspace = tempdir().expect("temp workspace");
        let project_dir = workspace.path().join("project");
        fs::create_dir_all(&project_dir).expect("create project folder");
        write_required_files(&project_dir);

        let mut app = PackerApp::default();
        app.folder = Some(project_dir.clone());
        app.folder_base_name = "SAVE".to_string();
        app.psu_file_base_name.clear();
        app.selected_prefix = SasPrefix::App;
        app.output.clear();

        let result = app.prepare_pack_inputs();
        assert!(result.is_some(), "inputs should prepare successfully");
        assert!(app.output.ends_with("APP_SAVE.psu"));
    }

    #[test]
    fn declining_pack_confirmation_keeps_warning_visible() {
        let workspace = tempdir().expect("temp workspace");
        let project_dir = workspace.path().join("project");
        fs::create_dir_all(&project_dir).expect("create project folder");

        let mut app = PackerApp::default();
        app.folder = Some(project_dir);
        app.folder_base_name = "SAVE".to_string();
        app.psu_file_base_name = "SAVE".to_string();
        app.selected_prefix = SasPrefix::App;
        app.output = workspace.path().join("output.psu").display().to_string();

        app.handle_pack_request();

        assert!(
            app.pending_pack_action.is_some(),
            "confirmation should be pending"
        );
        assert!(
            !app.missing_required_project_files.is_empty(),
            "missing files should be tracked"
        );

        let missing_before = app.missing_required_project_files.clone();
        app.cancel_pending_pack_action();

        assert!(
            app.pending_pack_action.is_none(),
            "pending confirmation cleared"
        );
        assert_eq!(
            app.missing_required_project_files, missing_before,
            "warning about missing files remains visible"
        );
    }

    #[test]
    fn accepting_pack_confirmation_triggers_pack_job() {
        let workspace = tempdir().expect("temp workspace");
        let project_dir = workspace.path().join("project");
        fs::create_dir_all(&project_dir).expect("create project folder");

        let mut app = PackerApp::default();
        app.folder = Some(project_dir);
        app.folder_base_name = "SAVE".to_string();
        app.psu_file_base_name = "SAVE".to_string();
        app.selected_prefix = SasPrefix::App;
        app.output = workspace.path().join("output.psu").display().to_string();

        app.handle_pack_request();
        assert!(
            app.pending_pack_action.is_some(),
            "confirmation should be pending"
        );
        assert!(!app.test_pack_job_started);

        app.confirm_pending_pack_action();

        assert!(app.pending_pack_action.is_none(), "confirmation accepted");
        assert!(
            app.test_pack_job_started,
            "pack job should start after acceptance"
        );
        assert!(app.pack_job.is_some(), "pack job handle should be created");

        wait_for_pack_completion(&mut app);
    }

    #[test]
    fn update_psu_overwrites_existing_file() {
        let workspace = tempdir().expect("temp workspace");
        let project_dir = workspace.path().join("project");
        fs::create_dir_all(&project_dir).expect("create project folder");
        write_required_files(&project_dir);

        let existing_output = workspace.path().join("existing.psu");
        fs::write(&existing_output, b"old").expect("create placeholder output");

        let mut app = PackerApp::default();
        app.folder = Some(project_dir);
        app.folder_base_name = "SAVE".to_string();
        app.psu_file_base_name = "SAVE".to_string();
        app.selected_prefix = SasPrefix::App;
        app.output = existing_output.display().to_string();
        app.loaded_psu_path = Some(existing_output.clone());

        app.handle_update_psu_request();

        assert!(app.pack_job.is_some(), "pack job should start");
        wait_for_pack_completion(&mut app);

        assert!(app.error_message.is_none(), "no error after update");
        assert!(app.status.contains(&existing_output.display().to_string()));
        let metadata = fs::metadata(&existing_output).expect("output metadata");
        assert!(metadata.len() > 0, "packed PSU should not be empty");
    }

    #[test]
    fn update_psu_reports_missing_destination() {
        let workspace = tempdir().expect("temp workspace");
        let project_dir = workspace.path().join("project");
        fs::create_dir_all(&project_dir).expect("create project folder");
        write_required_files(&project_dir);

        let missing_output = workspace.path().join("missing.psu");

        let mut app = PackerApp::default();
        app.folder = Some(project_dir);
        app.folder_base_name = "SAVE".to_string();
        app.psu_file_base_name = "SAVE".to_string();
        app.selected_prefix = SasPrefix::App;
        app.output = missing_output.display().to_string();
        app.loaded_psu_path = Some(missing_output.clone());

        app.handle_update_psu_request();

        assert!(app.pack_job.is_none(), "pack job should not start");
        let message = app.error_message.expect("error message expected");
        assert!(message.contains("does not exist"));
    }

    #[test]
    fn update_loaded_psu_without_project_folder_uses_temporary_workspace() {
        let workspace = tempdir().expect("temp workspace");
        let project_dir = workspace.path().join("project");
        fs::create_dir_all(&project_dir).expect("create project folder");
        write_required_files(&project_dir);

        let existing_output = workspace.path().join("existing.psu");
        let config = PsuConfig {
            name: "APP_SAVE".to_string(),
            timestamp: None,
            include: None,
            exclude: None,
            icon_sys: None,
        };
        psu_packer::pack_with_config(&project_dir, &existing_output, config)
            .expect("pack source PSU");

        let mut app = PackerApp::default();
        app.folder = None;
        app.folder_base_name = "SAVE".to_string();
        app.psu_file_base_name = "SAVE".to_string();
        app.selected_prefix = SasPrefix::App;
        app.output = existing_output.display().to_string();
        app.loaded_psu_path = Some(existing_output.clone());

        app.handle_update_psu_request();

        assert!(app.pack_job.is_some(), "pack job should start");
        assert_ne!(
            app.error_message.as_deref(),
            Some("Please select a folder"),
            "loaded PSU update should not emit folder selection error"
        );
        assert!(
            app.folder.is_none(),
            "temporary workspace should not persist as project folder"
        );

        wait_for_pack_completion(&mut app);

        assert!(
            app.error_message.is_none(),
            "no error after updating loaded PSU"
        );
        assert!(
            app.temp_workspace.is_none(),
            "temporary workspace should be cleaned up"
        );
    }

    #[test]
    fn export_psu_contents_to_folder() {
        let workspace = tempdir().expect("temp workspace");
        let project_dir = workspace.path().join("project");
        fs::create_dir_all(&project_dir).expect("create project folder");
        write_required_files(&project_dir);
        fs::write(project_dir.join("EXTRA.BIN"), b"payload").expect("write extra file");

        let psu_path = workspace.path().join("source.psu");
        let config = PsuConfig {
            name: "APP_SAVE".to_string(),
            timestamp: None,
            include: None,
            exclude: None,
            icon_sys: None,
        };
        psu_packer::pack_with_config(&project_dir, &psu_path, config).expect("pack source PSU");

        let export_parent = workspace.path().join("export");
        fs::create_dir_all(&export_parent).expect("create export parent");

        let app = PackerApp::default();
        let exported_root = app
            .export_psu_to_folder(&psu_path, &export_parent)
            .expect("export succeeds");

        assert_eq!(exported_root, export_parent.join("APP_SAVE"));
        assert!(
            !exported_root.join("psu.toml").exists(),
            "psu.toml should not be embedded in exported PSUs"
        );
        assert!(exported_root.join("title.cfg").exists());
        assert!(exported_root.join("icon.icn").exists());
        assert!(exported_root.join("EXTRA.BIN").exists());
    }

    #[test]
    fn export_psu_fails_for_missing_source() {
        let workspace = tempdir().expect("temp workspace");
        let destination = workspace.path();
        let app = PackerApp::default();

        let result = app.export_psu_to_folder(Path::new("/nonexistent.psu"), destination);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }
}

fn save_editor_to_disk(
    folder: Option<&Path>,
    file_name: &str,
    editor: &mut TextFileEditor,
) -> Result<PathBuf, io::Error> {
    let folder =
        folder.ok_or_else(|| io::Error::new(io::ErrorKind::Other, "No folder selected"))?;
    let path = folder.join(file_name);
    fs::write(&path, editor.content.as_bytes())?;
    editor.modified = false;
    editor.load_error = None;
    Ok(path)
}

#[derive(Default)]
struct TextEditorActions {
    save_clicked: bool,
    apply_clicked: bool,
}

fn editor_action_buttons(
    ui: &mut egui::Ui,
    file_name: &str,
    editing_enabled: bool,
    save_enabled: bool,
    editor: &mut TextFileEditor,
) -> TextEditorActions {
    let mut actions = TextEditorActions::default();

    if save_enabled {
        ui.horizontal(|ui| {
            let button_label = format!("Save {file_name}");
            if ui
                .add_enabled(editor.modified, egui::Button::new(button_label))
                .clicked()
            {
                actions.save_clicked = true;
            }

            if editor.modified {
                if ui
                    .add_enabled(
                        editor.modified,
                        egui::Button::new(format!("Apply {file_name}")),
                    )
                    .clicked()
                {
                    actions.apply_clicked = true;
                }
                ui.colored_label(egui::Color32::YELLOW, "Unsaved changes");
            }
        });
    } else if editing_enabled {
        if editor.modified {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        editor.modified,
                        egui::Button::new(format!("Apply {file_name}")),
                    )
                    .clicked()
                {
                    actions.apply_clicked = true;
                }
                ui.colored_label(egui::Color32::YELLOW, "Unsaved changes");
            });
        }
        ui.label(
            egui::RichText::new(format!(
                "Edits to {file_name} are kept in memory. Select a folder when you're ready to save them to disk."
            ))
            .italics(),
        );
    } else {
        ui.label(format!(
            "Select a folder or open a PSU to edit {file_name}."
        ));
    }

    actions
}

#[cfg(feature = "psu-toml-editor")]
fn text_editor_ui(
    ui: &mut egui::Ui,
    file_name: &str,
    editing_enabled: bool,
    save_enabled: bool,
    editor: &mut TextFileEditor,
) -> TextEditorActions {
    if let Some(message) = &editor.load_error {
        ui.colored_label(egui::Color32::YELLOW, message);
        ui.add_space(8.0);
    }

    let show_editor = editing_enabled || !editor.content.is_empty();

    if show_editor {
        let response = egui::ScrollArea::vertical()
            .id_source(format!("{file_name}_editor_scroll"))
            .show(ui, |ui| {
                ui.add_enabled(
                    editing_enabled,
                    egui::TextEdit::multiline(&mut editor.content)
                        .desired_rows(20)
                        .code_editor(),
                )
            })
            .inner;

        if editing_enabled && response.changed() {
            editor.modified = true;
        }
    }

    ui.add_space(8.0);
    editor_action_buttons(ui, file_name, editing_enabled, save_enabled, editor)
}

fn title_cfg_form_ui(
    ui: &mut egui::Ui,
    editing_enabled: bool,
    save_enabled: bool,
    editor: &mut TextFileEditor,
) -> TextEditorActions {
    if let Some(message) = &editor.load_error {
        ui.colored_label(egui::Color32::YELLOW, message);
        ui.add_space(8.0);
    }

    let show_form = editing_enabled || !editor.content.is_empty();

    if show_form {
        let previous_content = editor.content.clone();
        let mut cfg = TitleCfg::new(editor.content.clone());
        let helper = cfg.helper.clone();

        let mut keys: Vec<String> = cfg.index_map.keys().cloned().collect();
        let mut seen_keys: HashSet<String> = keys.iter().cloned().collect();
        for key in helper.keys() {
            if seen_keys.insert(key.clone()) {
                keys.push(key.clone());
            }
        }

        let missing_fields = cfg.missing_mandatory_fields();
        let missing_field_set: HashSet<&str> = missing_fields.iter().copied().collect();

        let mut section_lookup: HashMap<&'static str, usize> = HashMap::new();
        for (index, (_, field_keys)) in TITLE_CFG_SECTIONS.iter().enumerate() {
            for key in *field_keys {
                section_lookup.insert(*key, index);
            }
        }

        let mut section_fields: Vec<Vec<String>> = vec![Vec::new(); TITLE_CFG_SECTIONS.len()];
        let mut additional_fields: Vec<String> = Vec::new();
        for key in &keys {
            if let Some(&index) = section_lookup.get(key.as_str()) {
                section_fields[index].push(key.clone());
            } else {
                additional_fields.push(key.clone());
            }
        }

        let mut index_map_changed = false;

        egui::ScrollArea::vertical()
            .id_source("title_cfg_form_scroll")
            .show(ui, |ui| {
                ui::centered_column(ui, CENTERED_COLUMN_MAX_WIDTH, |ui| {
                    if !missing_fields.is_empty() {
                        let message =
                            format!("Missing mandatory fields: {}", missing_fields.join(", "));
                        ui.colored_label(egui::Color32::YELLOW, message);
                        ui.add_space(8.0);
                    }

                    let mut render_fields =
                        |ui: &mut egui::Ui, grid_id: String, section_keys: &[String]| {
                            egui::Grid::new(grid_id)
                                .num_columns(2)
                                .spacing(TITLE_CFG_GRID_SPACING)
                                .striped(true)
                                .show(ui, |ui| {
                                    for key in section_keys {
                                        let mut tooltip: Option<String> = None;
                                        let mut hint: Option<String> = None;
                                        let mut values: Option<Vec<String>> = None;
                                        let mut char_limit: Option<usize> = None;
                                        let mut multiline = false;

                                        if let Some(table) =
                                            helper.get(key).and_then(|value| value.as_table())
                                        {
                                            tooltip = table
                                                .get("tooltip")
                                                .and_then(|value| value.as_str())
                                                .map(|s| s.to_owned());
                                            hint = table
                                                .get("hint")
                                                .and_then(|value| value.as_str())
                                                .map(|s| s.to_owned());
                                            if let Some(array) = table
                                                .get("values")
                                                .and_then(|value| value.as_array())
                                            {
                                                let options: Vec<String> = array
                                                    .iter()
                                                    .filter_map(|value| {
                                                        value.as_str().map(|s| s.to_owned())
                                                    })
                                                    .collect();
                                                if !options.is_empty() {
                                                    values = Some(options);
                                                }
                                            }
                                            char_limit = table
                                                .get("char_limit")
                                                .and_then(|value| value.as_integer())
                                                .and_then(|value| {
                                                    (value >= 0).then(|| value as usize)
                                                });
                                            multiline = table
                                                .get("multiline")
                                                .and_then(|value| value.as_bool())
                                                .unwrap_or(false);
                                        }

                                        let mut label_text = egui::RichText::new(key.as_str());
                                        if missing_field_set.contains(key.as_str()) {
                                            label_text = label_text.color(egui::Color32::YELLOW);
                                        }
                                        let label = ui.label(label_text);
                                        if let Some(tooltip) = &tooltip {
                                            label.on_hover_text(tooltip);
                                        }

                                        let existing_value =
                                            cfg.index_map.get(key).cloned().unwrap_or_default();
                                        let mut new_value = existing_value.clone();
                                        let mut field_changed = false;

                                        if let Some(options) = values.as_ref() {
                                            let display_text = if new_value.is_empty() {
                                                hint.clone()
                                                    .unwrap_or_else(|| "(not set)".to_string())
                                            } else {
                                                new_value.clone()
                                            };
                                            if editing_enabled {
                                                let response = egui::ComboBox::from_id_source(
                                                    format!("title_cfg_option_{key}"),
                                                )
                                                .selected_text(display_text.clone())
                                                .show_ui(ui, |ui| {
                                                    ui.selectable_value(
                                                        &mut new_value,
                                                        String::new(),
                                                        "(not set)",
                                                    );
                                                    for option in options {
                                                        ui.selectable_value(
                                                            &mut new_value,
                                                            option.clone(),
                                                            option,
                                                        );
                                                    }
                                                });
                                                if let Some(tooltip) = &tooltip {
                                                    response.response.on_hover_text(tooltip);
                                                }
                                                if new_value != existing_value {
                                                    field_changed = true;
                                                }
                                            } else {
                                                let response = ui.label(display_text);
                                                if let Some(tooltip) = &tooltip {
                                                    response.on_hover_text(tooltip);
                                                }
                                            }
                                        } else {
                                            let mut text_edit = if multiline {
                                                egui::TextEdit::multiline(&mut new_value)
                                                    .desired_rows(TITLE_CFG_MULTILINE_ROWS)
                                                    .desired_width(f32::INFINITY)
                                            } else {
                                                egui::TextEdit::singleline(&mut new_value)
                                            };
                                            if let Some(hint) = &hint {
                                                text_edit = text_edit.hint_text(hint.clone());
                                            }
                                            if let Some(limit) = char_limit {
                                                text_edit = text_edit.char_limit(limit);
                                            }
                                            let response =
                                                ui.add_enabled(editing_enabled, text_edit);
                                            let changed = editing_enabled
                                                && response.changed()
                                                && new_value != existing_value;
                                            if let Some(tooltip) = &tooltip {
                                                response.on_hover_text(tooltip);
                                            }
                                            if changed {
                                                field_changed = true;
                                            }
                                        }

                                        if editing_enabled && field_changed {
                                            cfg.index_map.insert(key.clone(), new_value);
                                            index_map_changed = true;
                                        }

                                        ui.end_row();
                                    }
                                });
                        };

                    let mut rendered_section = false;
                    for (index, (title, _)) in TITLE_CFG_SECTIONS.iter().enumerate() {
                        let section_keys = &section_fields[index];
                        if section_keys.is_empty() {
                            continue;
                        }
                        if rendered_section {
                            ui.add_space(TITLE_CFG_SECTION_GAP);
                        }
                        rendered_section = true;
                        ui.heading(theme::display_heading_text(ui, *title));
                        ui.add_space(TITLE_CFG_SECTION_HEADING_GAP);
                        render_fields(ui, format!("title_cfg_form_grid_{title}"), section_keys);
                    }

                    if !additional_fields.is_empty() {
                        if rendered_section {
                            ui.add_space(TITLE_CFG_SECTION_GAP);
                        }
                        ui.heading(theme::display_heading_text(ui, "Additional fields"));
                        ui.add_space(TITLE_CFG_SECTION_HEADING_GAP);
                        render_fields(
                            ui,
                            "title_cfg_form_grid_additional".to_string(),
                            &additional_fields,
                        );
                    }
                });
            });

        if index_map_changed {
            cfg.sync_index_map_to_contents();
            if cfg.contents != previous_content {
                editor.content = cfg.contents.clone();
                editor.modified = true;
            }
        }
    }

    ui.add_space(8.0);
    editor_action_buttons(ui, "title.cfg", editing_enabled, save_enabled, editor)
}

impl eframe::App for PackerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_pack_job();

        if ctx.input(|i| i.viewport().close_requested()) && !self.exit_confirmed {
            self.exit_confirmed = false;
            self.show_exit_confirm = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }

        self.zoom_factor = self.zoom_factor.clamp(0.5, 2.0);
        ctx.set_pixels_per_point(self.zoom_factor);

        let source_present = self.has_source();
        if !source_present && self.source_present_last_frame {
            self.reset_metadata_fields();
        }
        self.source_present_last_frame = source_present;

        if let Some(progress) = self.pack_progress_value() {
            ctx.request_repaint();
            egui::Window::new("packing_progress")
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .frame(egui::Frame::popup(&ctx.style()))
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("Packing PSU…")
                                .font(theme::display_font(26.0))
                                .color(self.theme.neon_accent),
                        );
                        ui.add_space(8.0);
                        ui.add(
                            egui::ProgressBar::new(progress)
                                .desired_width(200.0)
                                .animate(true),
                        );
                    });
                });
        }

        egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::none().fill(self.theme.background))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                theme::draw_vertical_gradient(
                    ui.painter(),
                    rect,
                    self.theme.header_top,
                    self.theme.header_bottom,
                );
                let separator_rect =
                    egui::Rect::from_min_max(egui::pos2(rect.min.x, rect.max.y - 2.0), rect.max);
                theme::draw_separator(ui.painter(), separator_rect, self.theme.separator);
                egui::menu::bar(ui, |ui| {
                    ui::file_picker::file_menu(self, ui);
                    ui.menu_button("View", |ui| {
                        if ui.button("Zoom In").clicked() {
                            self.zoom_factor = (self.zoom_factor + 0.1).min(2.0);
                            ui.close_menu();
                        }
                        if ui.button("Zoom Out").clicked() {
                            self.zoom_factor = (self.zoom_factor - 0.1).max(0.5);
                            ui.close_menu();
                        }
                        if ui.button("Reset Zoom").clicked() {
                            self.zoom_factor = 1.0;
                            ui.close_menu();
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(12.0);
                        let zoom_text = format!("Zoom: {:.0}%", self.zoom_factor * 100.0);
                        ui.label(egui::RichText::new(zoom_text).color(self.theme.neon_accent));
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(self.theme.background))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                theme::draw_vertical_gradient(
                    ui.painter(),
                    rect,
                    self.theme.footer_top,
                    self.theme.footer_bottom,
                );
                let top_separator =
                    egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.min.y + 2.0));
                theme::draw_separator(ui.painter(), top_separator, self.theme.separator);
                let bottom_separator =
                    egui::Rect::from_min_max(egui::pos2(rect.min.x, rect.max.y - 2.0), rect.max);
                theme::draw_separator(ui.painter(), bottom_separator, self.theme.separator);
                ui.add_space(8.0);

                let tab_font = theme::display_font(18.0);
                let tab_bar = ui.horizontal_wrapped(|ui| {
                    let spacing = ui.spacing_mut();
                    spacing.item_spacing.x = 12.0;
                    spacing.item_spacing.y = 8.0;

                    self.editor_tab_button(
                        ui,
                        EditorTab::PsuSettings,
                        "PSU Settings",
                        false,
                        &tab_font,
                    );

                    #[cfg(feature = "psu-toml-editor")]
                    {
                        let psu_toml_label = if self.psu_toml_editor.modified {
                            "psu.toml*"
                        } else {
                            "psu.toml"
                        };
                        self.editor_tab_button(
                            ui,
                            EditorTab::PsuToml,
                            psu_toml_label,
                            self.psu_toml_editor.modified,
                            &tab_font,
                        );
                    }

                    let title_cfg_label = if self.title_cfg_editor.modified {
                        "title.cfg*"
                    } else {
                        "title.cfg"
                    };
                    self.editor_tab_button(
                        ui,
                        EditorTab::TitleCfg,
                        title_cfg_label,
                        self.title_cfg_editor.modified,
                        &tab_font,
                    );

                    self.editor_tab_button(ui, EditorTab::IconSys, "icon.sys", false, &tab_font);

                    let timestamp_label = if self.timestamp_rules_modified {
                        "Timestamp rules*"
                    } else {
                        "Timestamp rules"
                    };
                    self.editor_tab_button(
                        ui,
                        EditorTab::TimestampAuto,
                        timestamp_label,
                        self.timestamp_rules_modified,
                        &tab_font,
                    );
                });

                let tab_rect = tab_bar.response.rect;
                let tab_separator = egui::Rect::from_min_max(
                    egui::pos2(rect.min.x, tab_rect.max.y + 4.0),
                    egui::pos2(rect.max.x, tab_rect.max.y + 6.0),
                );
                theme::draw_separator(ui.painter(), tab_separator, self.theme.separator);
                ui.add_space(10.0);

                match self.editor_tab {
                    EditorTab::PsuSettings => {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui::centered_column(ui, CENTERED_COLUMN_MAX_WIDTH, |ui| {
                                ui::file_picker::folder_section(self, ui);

                                let showing_psu = self.showing_loaded_psu();
                                if showing_psu {
                                    ui.add_space(8.0);
                                    ui::file_picker::loaded_psu_section(self, ui);
                                }

                                ui.add_space(8.0);

                                let two_column_layout =
                                    ui.available_width() >= PACK_CONTROLS_TWO_COLUMN_MIN_WIDTH;
                                if two_column_layout {
                                    ui.columns(2, |columns| {
                                        columns[0].vertical(|ui| {
                                            ui::pack_controls::metadata_section(self, ui);
                                            ui.add_space(8.0);
                                            ui::pack_controls::output_section(self, ui);
                                        });

                                        columns[1].vertical(|ui| {
                                            if !showing_psu {
                                                ui::pack_controls::file_filters_section(self, ui);
                                                ui.add_space(8.0);
                                            }
                                            ui::pack_controls::packaging_section(self, ui);
                                        });
                                    });
                                } else {
                                    ui::pack_controls::metadata_section(self, ui);

                                    if !showing_psu {
                                        ui.add_space(8.0);
                                        ui::pack_controls::file_filters_section(self, ui);
                                    }

                                    ui.add_space(8.0);
                                    ui::pack_controls::output_section(self, ui);

                                    ui.add_space(8.0);
                                    ui::pack_controls::packaging_section(self, ui);
                                }
                            });
                        });
                    }
                    #[cfg(feature = "psu-toml-editor")]
                    EditorTab::PsuToml => {
                        let editing_enabled = true; // Allow editing even without a source selection.
                        let save_enabled = self.folder.is_some();
                        let actions = text_editor_ui(
                            ui,
                            "psu.toml",
                            editing_enabled,
                            save_enabled,
                            &mut self.psu_toml_editor,
                        );
                        if actions.save_clicked {
                            match save_editor_to_disk(
                                self.folder.as_deref(),
                                "psu.toml",
                                &mut self.psu_toml_editor,
                            ) {
                                Ok(path) => {
                                    self.status = format!("Saved {}", path.display());
                                    self.clear_error_message();
                                }
                                Err(err) => {
                                    self.set_error_message(format!(
                                        "Failed to save psu.toml: {err}"
                                    ));
                                }
                            }
                        }
                        if actions.apply_clicked {
                            self.apply_psu_toml_edits();
                        }
                    }
                    EditorTab::TitleCfg => {
                        let editing_enabled = true; // Allow editing even without a source selection.
                        let save_enabled = self.folder.is_some();
                        let actions = title_cfg_form_ui(
                            ui,
                            editing_enabled,
                            save_enabled,
                            &mut self.title_cfg_editor,
                        );
                        if actions.save_clicked {
                            match save_editor_to_disk(
                                self.folder.as_deref(),
                                "title.cfg",
                                &mut self.title_cfg_editor,
                            ) {
                                Ok(path) => {
                                    self.status = format!("Saved {}", path.display());
                                    self.clear_error_message();
                                }
                                Err(err) => {
                                    self.set_error_message(format!(
                                        "Failed to save title.cfg: {err}"
                                    ));
                                }
                            }
                        }
                        if actions.apply_clicked {
                            self.apply_title_cfg_edits();
                        }
                    }
                    EditorTab::IconSys => {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui::centered_column(ui, CENTERED_COLUMN_MAX_WIDTH, |ui| {
                                ui::icon_sys::icon_sys_editor(self, ui);
                            });
                        });
                    }
                    EditorTab::TimestampAuto => {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui::centered_column(ui, CENTERED_COLUMN_MAX_WIDTH, |ui| {
                                ui::timestamps::timestamp_rules_editor(self, ui);
                            });
                        });
                    }
                }
            });

        ui::dialogs::pack_confirmation(self, ctx);
        ui::dialogs::exit_confirmation(self, ctx);
    }
}

struct EditorTabWidget<'a> {
    label: &'a str,
    font: egui::FontId,
    theme: &'a theme::Palette,
    is_selected: bool,
    alert: bool,
}

impl<'a> EditorTabWidget<'a> {
    fn new(
        label: &'a str,
        font: egui::FontId,
        theme: &'a theme::Palette,
        is_selected: bool,
        alert: bool,
    ) -> Self {
        Self {
            label,
            font,
            theme,
            is_selected,
            alert,
        }
    }
}

impl<'a> Widget for EditorTabWidget<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let base_padding = egui::vec2(12.0, 6.0);
        let hover_extra = egui::vec2(2.0, 2.0);
        let selected_extra = egui::vec2(4.0, 4.0);
        let max_padding = base_padding + selected_extra;
        let rounding = egui::CornerRadius::same(10);

        let mut text_color = self.theme.text_primary;
        if self.is_selected {
            text_color = egui::Color32::WHITE;
        } else if self.alert {
            text_color = self.theme.neon_accent;
        }

        let galley = ui.fonts(|fonts| {
            fonts.layout_no_wrap(self.label.to_owned(), self.font.clone(), text_color)
        });
        let desired_size = galley.size() + max_padding * 2.0;

        let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let mut padding = base_padding;
            if response.hovered() {
                padding += hover_extra;
            }
            if self.is_selected {
                padding += selected_extra;
            }

            let fill = if self.is_selected {
                self.theme.neon_accent.gamma_multiply(0.45)
            } else if response.hovered() {
                self.theme.soft_accent.gamma_multiply(0.38)
            } else if self.alert {
                self.theme.neon_accent.gamma_multiply(0.24)
            } else {
                self.theme.soft_accent.gamma_multiply(0.24)
            };

            let mut stroke_color = self.theme.soft_accent.gamma_multiply(0.7);
            if self.is_selected {
                stroke_color = self.theme.neon_accent;
            } else if self.alert || response.hovered() {
                stroke_color = self.theme.neon_accent.gamma_multiply(0.8);
            }

            ui.painter().rect_filled(rect, rounding, fill);
            ui.painter().rect_stroke(
                rect,
                rounding,
                egui::Stroke::new(1.0, stroke_color),
                egui::StrokeKind::Outside,
            );

            let text_pos = rect.left_top() + padding;
            ui.painter().galley(text_pos, galley, text_color);
        }

        response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        let enabled = response.enabled();
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, self.label)
        });
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ps2_filetypes::sjis;
    use psu_packer::IconSysFlags;
    use std::fs;
    use tempfile::tempdir;

    #[cfg(feature = "psu-toml-editor")]
    #[test]
    fn manual_edits_persist_without_folder_selection() {
        let mut app = PackerApp::default();
        app.open_psu_toml_tab();

        app.psu_toml_editor
            .set_content("custom configuration".to_string());
        app.psu_toml_editor.modified = true;

        let ctx = egui::Context::default();

        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let actions = text_editor_ui(
                    ui,
                    "psu.toml",
                    true,
                    app.folder.is_some(),
                    &mut app.psu_toml_editor,
                );
                assert!(!actions.save_clicked);
                assert!(!actions.apply_clicked);
            });
        });

        assert_eq!(app.psu_toml_editor.content, "custom configuration");
        assert!(app.psu_toml_editor.modified);

        app.open_title_cfg_tab();
        app.title_cfg_editor
            .set_content("title settings".to_string());
        app.title_cfg_editor.modified = true;

        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let actions =
                    title_cfg_form_ui(ui, true, app.folder.is_some(), &mut app.title_cfg_editor);
                assert!(!actions.save_clicked);
                assert!(!actions.apply_clicked);
            });
        });

        assert_eq!(app.psu_toml_editor.content, "custom configuration");
        assert!(app.psu_toml_editor.modified);

        app.open_psu_toml_tab();

        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let actions = text_editor_ui(
                    ui,
                    "psu.toml",
                    true,
                    app.folder.is_some(),
                    &mut app.psu_toml_editor,
                );
                assert!(!actions.save_clicked);
                assert!(!actions.apply_clicked);
            });
        });

        assert_eq!(app.psu_toml_editor.content, "custom configuration");
        assert!(app.psu_toml_editor.modified);
    }

    #[cfg(feature = "psu-toml-editor")]
    #[test]
    fn apply_psu_toml_updates_state_without_disk() {
        let mut app = PackerApp::default();
        let timestamp = "2023-05-17 08:30:00";
        app.psu_toml_editor.content = format!(
            r#"[config]
name = "APP_Custom Save"
timestamp = "{timestamp}"
include = ["BOOT.ELF", "DATA.BIN"]
exclude = ["IGNORE.DAT"]

[icon_sys]
flags = 1
title = "HELLOWORLD"
linebreak_pos = 5
"#
        );
        app.psu_toml_editor.modified = true;

        assert!(app.apply_psu_toml_edits());

        assert_eq!(app.selected_prefix, SasPrefix::App);
        assert_eq!(app.folder_base_name, "Custom Save");
        assert_eq!(app.psu_file_base_name, "Custom Save");
        assert_eq!(app.include_files, vec!["BOOT.ELF", "DATA.BIN"]);
        assert_eq!(app.exclude_files, vec!["IGNORE.DAT"]);
        let expected_timestamp =
            NaiveDateTime::parse_from_str(timestamp, TIMESTAMP_FORMAT).unwrap();
        assert_eq!(app.timestamp, Some(expected_timestamp));
        assert_eq!(app.timestamp_strategy, TimestampStrategy::Manual);
        assert!(app.icon_sys_enabled);
        assert!(matches!(
            app.icon_sys_flag_selection,
            IconFlagSelection::Preset(1)
        ));
        assert_eq!(app.icon_sys_custom_flag, 1);
        assert_eq!(app.icon_sys_title_line1, "HELLO");
        assert_eq!(app.icon_sys_title_line2, "WORLD");
        assert!(!app.psu_toml_sync_blocked);
        assert!(app.psu_toml_editor.modified);
    }

    #[test]
    fn apply_icon_sys_file_preserves_multibyte_characters() {
        let mut app = PackerApp::default();
        let title = "メモリーカード";

        let icon_sys = IconSys {
            flags: 4,
            linebreak_pos: sjis::encode_sjis("メモリー").unwrap().len() as u16,
            background_transparency: IconSysConfig::default_background_transparency(),
            background_colors: IconSysConfig::default_background_colors().map(Into::into),
            light_directions: IconSysConfig::default_light_directions().map(Into::into),
            light_colors: IconSysConfig::default_light_colors().map(Into::into),
            ambient_color: IconSysConfig::default_ambient_color().into(),
            title: title.to_string(),
            icon_file: "icon.icn".to_string(),
            icon_copy_file: "icon.icn".to_string(),
            icon_delete_file: "icon.icn".to_string(),
        };

        app.apply_icon_sys_file(&icon_sys);

        assert_eq!(app.icon_sys_title_line1, "メモリー");
        assert_eq!(app.icon_sys_title_line2, "カード");
    }

    #[test]
    fn apply_icon_sys_config_preserves_multibyte_characters() {
        let mut app = PackerApp::default();
        let title = "メモリーカード";

        let icon_cfg = IconSysConfig {
            flags: IconSysFlags::new(1),
            title: title.to_string(),
            linebreak_pos: Some(sjis::encode_sjis("メモリー").unwrap().len() as u16),
            preset: None,
            background_transparency: None,
            background_colors: None,
            light_directions: None,
            light_colors: None,
            ambient_color: None,
        };

        app.apply_icon_sys_config(icon_cfg, None);

        assert_eq!(app.icon_sys_title_line1, "メモリー");
        assert_eq!(app.icon_sys_title_line2, "カード");
    }

    #[test]
    fn load_project_files_reads_uppercase_icon_sys() {
        use ps2_filetypes::{color::Color, ColorF, Vector};

        let temp_dir = tempdir().expect("temporary directory");
        let folder = temp_dir.path();

        let config = psu_packer::Config {
            name: "APP_Test Save".to_string(),
            timestamp: None,
            include: None,
            exclude: None,
            icon_sys: None,
        };
        let config_toml = config.to_toml_string().expect("serialize minimal psu.toml");
        fs::write(folder.join("psu.toml"), config_toml).expect("write psu.toml");
        fs::write(folder.join("title.cfg"), "title=Test Save\n").expect("write title.cfg");

        let icon_sys = IconSys {
            flags: 1,
            linebreak_pos: 5,
            background_transparency: 0,
            background_colors: [Color::WHITE; 4],
            light_directions: [
                Vector {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                    w: 0.0,
                },
                Vector {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                    w: 0.0,
                },
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
            ],
            light_colors: [
                ColorF {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                },
                ColorF {
                    r: 0.5,
                    g: 0.5,
                    b: 0.5,
                    a: 1.0,
                },
                ColorF {
                    r: 0.25,
                    g: 0.25,
                    b: 0.25,
                    a: 1.0,
                },
            ],
            ambient_color: ColorF {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 1.0,
            },
            title: "HELLOWORLD".to_string(),
            icon_file: "icon.icn".to_string(),
            icon_copy_file: "icon.icn".to_string(),
            icon_delete_file: "icon.icn".to_string(),
        };
        let icon_bytes = icon_sys.to_bytes().expect("serialize icon.sys");
        fs::write(folder.join("ICON.SYS"), icon_bytes).expect("write ICON.SYS");

        let mut app = PackerApp::default();
        crate::ui::file_picker::load_project_files(&mut app, folder);

        assert!(app.icon_sys_existing.is_some());
        assert!(app.icon_sys_use_existing);
        assert_eq!(app.icon_sys_title_line1, "HELLO");
        assert_eq!(app.icon_sys_title_line2, "WORLD");
    }

    #[test]
    fn split_icon_sys_title_replaces_control_characters() {
        let (line1, line2) = split_icon_sys_title("A\u{0001}B\rC", 3);

        assert_eq!(
            line1,
            format!("A{}B", ICON_SYS_UNSUPPORTED_CHAR_PLACEHOLDER)
        );
        assert_eq!(line2, format!("{}C", ICON_SYS_UNSUPPORTED_CHAR_PLACEHOLDER));
    }

    #[test]
    fn split_icon_sys_title_uses_byte_based_breaks_for_multibyte_titles() {
        let title = "メモリーカード";
        let break_bytes = sjis::encode_sjis("メモリー").unwrap().len();

        let (line1, line2) = split_icon_sys_title(title, break_bytes);

        assert_eq!(line1, "メモリー");
        assert_eq!(line2, "カード");
    }

    #[test]
    fn split_icon_sys_title_preserves_second_line_for_partial_multibyte_breaks() {
        let title = "メモリーカード";
        let break_bytes = sjis::encode_sjis("メモ").unwrap().len() + 1;

        let (line1, line2) = split_icon_sys_title(title, break_bytes);

        assert_eq!(line1, "メモ");
        assert_eq!(line2, "リーカード");
    }

    #[cfg(feature = "psu-toml-editor")]
    #[test]
    fn apply_invalid_psu_toml_reports_error() {
        let mut app = PackerApp::default();
        app.psu_toml_editor.content = "[config".to_string();
        app.psu_toml_editor.modified = true;

        assert!(!app.apply_psu_toml_edits());
        assert!(app
            .error_message
            .as_ref()
            .is_some_and(|message| message.contains("Failed to")));
    }

    #[test]
    fn apply_title_cfg_validates_contents() {
        let mut app = PackerApp::default();
        app.title_cfg_editor.content = templates::TITLE_CFG_TEMPLATE.to_string();
        app.title_cfg_editor.modified = true;

        assert!(app.apply_title_cfg_edits());
        assert_eq!(app.status, "Validated title.cfg contents.");
        assert!(app.error_message.is_none());
    }

    #[test]
    fn apply_title_cfg_reports_missing_fields() {
        let mut app = PackerApp::default();
        app.title_cfg_editor.content = "title=Example".to_string();
        app.title_cfg_editor.modified = true;

        assert!(!app.apply_title_cfg_edits());
        assert!(app
            .error_message
            .as_ref()
            .is_some_and(|message| message.contains("missing mandatory")));
    }

    #[test]
    fn load_warning_flags_missing_required_files() {
        let temp_dir = tempdir().expect("temporary directory");
        for file in REQUIRED_PROJECT_FILES {
            let path = temp_dir.path().join(file);
            fs::write(&path, b"placeholder").expect("create required file");
        }

        let mut app = PackerApp::default();
        app.folder = Some(temp_dir.path().to_path_buf());

        app.refresh_missing_required_project_files();
        assert!(app.missing_required_project_files.is_empty());

        for file in REQUIRED_PROJECT_FILES {
            let path = temp_dir.path().join(file);
            fs::remove_file(&path).expect("remove required file");
            app.refresh_missing_required_project_files();
            assert_eq!(
                app.missing_required_project_files,
                vec![MissingRequiredFile::always(file)]
            );
            fs::write(&path, b"placeholder").expect("restore required file");
            app.refresh_missing_required_project_files();
            assert!(app.missing_required_project_files.is_empty());
        }

        // Optional files should only be required when their features are enabled.
        app.include_files.push("BOOT.ELF".to_string());
        app.refresh_missing_required_project_files();
        assert_eq!(
            app.missing_required_project_files,
            vec![MissingRequiredFile::included("BOOT.ELF")]
        );

        let boot_path = temp_dir.path().join("BOOT.ELF");
        fs::write(&boot_path, b"boot").expect("create BOOT.ELF");
        app.refresh_missing_required_project_files();
        assert!(app.missing_required_project_files.is_empty());

        let timestamp_path = temp_dir.path().join(TIMESTAMP_RULES_FILE);
        if timestamp_path.exists() {
            fs::remove_file(&timestamp_path).expect("remove timestamp rules");
        }

        app.timestamp_strategy = TimestampStrategy::SasRules;
        app.refresh_missing_required_project_files();
        assert!(app.missing_required_project_files.is_empty());

        app.mark_timestamp_rules_modified();
        app.refresh_missing_required_project_files();
        assert_eq!(
            app.missing_required_project_files,
            vec![MissingRequiredFile::timestamp_rules()]
        );

        app.timestamp_rules_modified = false;
        app.timestamp_rules_loaded_from_file = false;

        fs::write(&timestamp_path, b"{}").expect("create timestamp rules");
        app.load_timestamp_rules_from_folder(temp_dir.path());
        fs::remove_file(&timestamp_path).expect("remove timestamp rules");
        app.refresh_missing_required_project_files();
        assert_eq!(
            app.missing_required_project_files,
            vec![MissingRequiredFile::timestamp_rules()]
        );

        fs::write(&timestamp_path, b"{}").expect("restore timestamp rules");
        app.refresh_missing_required_project_files();
        assert!(app.missing_required_project_files.is_empty());
    }

    #[test]
    fn pack_request_blocks_missing_required_files() {
        let temp_dir = tempdir().expect("temporary directory");
        for file in REQUIRED_PROJECT_FILES {
            let path = temp_dir.path().join(file);
            fs::write(&path, b"placeholder").expect("create required file");
        }

        let mut app = PackerApp::default();
        app.folder = Some(temp_dir.path().to_path_buf());
        app.folder_base_name = "Sample".to_string();
        app.psu_file_base_name = "Sample".to_string();
        app.output = temp_dir.path().join("Sample.psu").display().to_string();

        for file in REQUIRED_PROJECT_FILES {
            let path = temp_dir.path().join(file);
            fs::remove_file(&path).expect("remove required file");
            app.handle_pack_request();
            let error = app
                .error_message
                .as_ref()
                .expect("missing files should block packing");
            assert!(error.contains(file));
            assert_eq!(
                app.missing_required_project_files,
                vec![MissingRequiredFile::always(file)]
            );
            fs::write(&path, b"placeholder").expect("restore required file");
            app.clear_error_message();
            app.refresh_missing_required_project_files();
            assert!(app.missing_required_project_files.is_empty());
        }

        // BOOT.ELF becomes required when referenced in the include list.
        let boot_path = temp_dir.path().join("BOOT.ELF");
        if boot_path.exists() {
            fs::remove_file(&boot_path).expect("remove BOOT.ELF");
        }
        app.include_files.push("BOOT.ELF".to_string());
        app.handle_pack_request();
        let error = app
            .error_message
            .as_ref()
            .expect("missing BOOT.ELF should block packing");
        assert!(error.contains("BOOT.ELF"));
        assert_eq!(
            app.missing_required_project_files,
            vec![MissingRequiredFile::included("BOOT.ELF")]
        );
        fs::write(&boot_path, b"boot").expect("restore BOOT.ELF");
        app.clear_error_message();
        app.refresh_missing_required_project_files();
        assert!(app.missing_required_project_files.is_empty());

        // Timestamp automation requires timestamp_rules.json when enabled.
        let timestamp_path = temp_dir.path().join(TIMESTAMP_RULES_FILE);
        if timestamp_path.exists() {
            fs::remove_file(&timestamp_path).expect("remove timestamp rules");
        }
        app.timestamp_strategy = TimestampStrategy::SasRules;
        let result = app.prepare_pack_inputs();
        assert!(
            result.is_some(),
            "timestamp automation should use built-in rules"
        );
        assert!(app.error_message.is_none());
        assert!(app.missing_required_project_files.is_empty());
    }
}
