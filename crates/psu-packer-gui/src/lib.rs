use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use chrono::NaiveDateTime;
use eframe::egui;
use ps2_filetypes::{templates, IconSys};
use psu_packer::{ColorConfig, ColorFConfig, IconSysConfig, VectorConfig};

pub(crate) mod sas_timestamps;
pub mod ui;

use sas_timestamps::TimestampRules;

pub use ui::{dialogs, file_picker, pack_controls};

pub(crate) const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
pub(crate) const ICON_SYS_FLAG_OPTIONS: &[(u16, &str)] =
    &[(0, "Save Data"), (1, "System Software"), (4, "Settings")];
pub(crate) const ICON_SYS_TITLE_CHAR_LIMIT: usize = 16;
const TIMESTAMP_RULES_FILE: &str = "timestamp_rules.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SasPrefix {
    None,
    App,
    Apps,
    Ps1,
    Ps2,
    Ps3,
    Ps4,
    Ps5,
    Psp,
    Psv,
}

pub(crate) const SAS_PREFIXES: [SasPrefix; 9] = [
    SasPrefix::App,
    SasPrefix::Apps,
    SasPrefix::Ps1,
    SasPrefix::Ps2,
    SasPrefix::Ps3,
    SasPrefix::Ps4,
    SasPrefix::Ps5,
    SasPrefix::Psp,
    SasPrefix::Psv,
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
            SasPrefix::Ps2 => "PS2_",
            SasPrefix::Ps3 => "PS3_",
            SasPrefix::Ps4 => "PS4_",
            SasPrefix::Ps5 => "PS5_",
            SasPrefix::Psp => "PSP_",
            SasPrefix::Psv => "PSV_",
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditorTab {
    PsuSettings,
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

#[derive(Clone, Default)]
pub(crate) struct TimestampRulesUiState {
    pub(crate) alias_texts: Vec<String>,
}

impl TimestampRulesUiState {
    pub(crate) fn from_rules(rules: &TimestampRules) -> Self {
        Self {
            alias_texts: rules
                .categories
                .iter()
                .map(|category| category.aliases.join("\n"))
                .collect(),
        }
    }

    pub(crate) fn ensure_matches(&mut self, rules: &TimestampRules) {
        if self.alias_texts.len() != rules.categories.len() {
            *self = Self::from_rules(rules);
        }
    }

    fn swap(&mut self, a: usize, b: usize) {
        if a >= self.alias_texts.len() || b >= self.alias_texts.len() {
            return;
        }
        if a == b {
            return;
        }
        self.alias_texts.swap(a, b);
    }
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
    pub(crate) timestamp_from_rules: bool,
    pub(crate) timestamp_rules: TimestampRules,
    pub(crate) timestamp_rules_modified: bool,
    pub(crate) timestamp_rules_error: Option<String>,
    pub(crate) timestamp_rules_ui: TimestampRulesUiState,
    pub(crate) include_files: Vec<String>,
    pub(crate) exclude_files: Vec<String>,
    pub(crate) selected_include: Option<usize>,
    pub(crate) selected_exclude: Option<usize>,
    pub(crate) loaded_psu_path: Option<PathBuf>,
    pub(crate) loaded_psu_files: Vec<String>,
    pub(crate) show_exit_confirm: bool,
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
    pack_job: Option<PackJob>,
    editor_tab: EditorTab,
    psu_toml_editor: TextFileEditor,
    title_cfg_editor: TextFileEditor,
    psu_toml_sync_blocked: bool,
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
            timestamp_from_rules: false,
            timestamp_rules,
            timestamp_rules_modified: false,
            timestamp_rules_error: None,
            timestamp_rules_ui,
            include_files: Vec::new(),
            exclude_files: Vec::new(),
            selected_include: None,
            selected_exclude: None,
            loaded_psu_path: None,
            loaded_psu_files: Vec::new(),
            show_exit_confirm: false,
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
            pack_job: None,
            editor_tab: EditorTab::PsuSettings,
            psu_toml_editor: TextFileEditor::default(),
            title_cfg_editor: TextFileEditor::default(),
            psu_toml_sync_blocked: false,
        }
    }
}

impl PackerApp {
    fn timestamp_rules_path_from(folder: &Path) -> PathBuf {
        folder.join(TIMESTAMP_RULES_FILE)
    }

    pub(crate) fn timestamp_rules_path(&self) -> Option<PathBuf> {
        self.folder
            .as_ref()
            .map(|folder| Self::timestamp_rules_path_from(folder))
    }

    pub(crate) fn load_timestamp_rules_from_folder(&mut self, folder: &Path) {
        let path = Self::timestamp_rules_path_from(folder);
        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<TimestampRules>(&content) {
                Ok(mut rules) => {
                    rules.sanitize();
                    self.timestamp_rules = rules;
                    self.timestamp_rules_error = None;
                }
                Err(err) => {
                    self.timestamp_rules = TimestampRules::default();
                    self.timestamp_rules_error =
                        Some(format!("Failed to parse {}: {err}", path.display()));
                }
            },
            Err(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    self.timestamp_rules = TimestampRules::default();
                    self.timestamp_rules_error = None;
                } else {
                    self.timestamp_rules = TimestampRules::default();
                    self.timestamp_rules_error =
                        Some(format!("Failed to read {}: {err}", path.display()));
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
        Ok(path)
    }

    pub(crate) fn mark_timestamp_rules_modified(&mut self) {
        self.timestamp_rules_modified = true;
        self.recompute_timestamp_from_rules();
    }

    fn recompute_timestamp_from_rules(&mut self) {
        if !self.timestamp_from_rules {
            return;
        }

        let Some(folder) = self.folder.as_ref() else {
            return;
        };

        let planned =
            sas_timestamps::planned_timestamp_for_folder(folder.as_path(), &self.timestamp_rules);
        if self.timestamp != planned {
            self.timestamp = planned;
            self.timestamp_from_rules = planned.is_some();
            self.refresh_psu_toml_editor();
        }
    }

    pub(crate) fn apply_planned_timestamp(&mut self) {
        let Some(folder) = self.folder.as_ref() else {
            self.timestamp_from_rules = false;
            return;
        };

        let planned =
            sas_timestamps::planned_timestamp_for_folder(folder.as_path(), &self.timestamp_rules);
        self.timestamp = planned;
        self.timestamp_from_rules = planned.is_some();
        self.refresh_psu_toml_editor();
    }

    pub(crate) fn planned_timestamp_for_current_folder(&self) -> Option<NaiveDateTime> {
        let folder = self.folder.as_ref()?;
        sas_timestamps::planned_timestamp_for_folder(folder.as_path(), &self.timestamp_rules)
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
            if category.aliases != aliases {
                category.aliases = aliases;
                self.mark_timestamp_rules_modified();
            }
        }
    }

    pub(crate) fn reset_timestamp_rules_to_default(&mut self) {
        self.timestamp_rules = TimestampRules::default();
        self.timestamp_rules_error = None;
        self.timestamp_rules_ui = TimestampRulesUiState::from_rules(&self.timestamp_rules);
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

        let ascii_chars: Vec<char> = icon_cfg
            .title
            .chars()
            .filter(|c| c.is_ascii() && *c != '\n' && *c != '\r')
            .collect();
        let break_index = icon_cfg.linebreak_position() as usize;
        let break_index = break_index.min(ascii_chars.len());
        let line1_count = break_index.min(ICON_SYS_TITLE_CHAR_LIMIT);
        let skip_count = line1_count;
        self.icon_sys_title_line1 = ascii_chars.iter().take(line1_count).copied().collect();
        self.icon_sys_title_line2 = ascii_chars
            .iter()
            .skip(skip_count)
            .take(ICON_SYS_TITLE_CHAR_LIMIT)
            .copied()
            .collect();

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

        let ascii_chars: Vec<char> = icon_sys
            .title
            .chars()
            .filter(|c| c.is_ascii() && *c != '\n' && *c != '\r')
            .collect();
        let break_index = (icon_sys.linebreak_pos as usize).min(ascii_chars.len());
        let line1_count = break_index.min(ICON_SYS_TITLE_CHAR_LIMIT);
        let skip_count = line1_count;
        self.icon_sys_title_line1 = ascii_chars.iter().take(line1_count).copied().collect();
        self.icon_sys_title_line2 = ascii_chars
            .iter()
            .skip(skip_count)
            .take(ICON_SYS_TITLE_CHAR_LIMIT)
            .copied()
            .collect();

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
        self.timestamp_from_rules = false;
        self.include_files.clear();
        self.exclude_files.clear();
        self.selected_include = None;
        self.selected_exclude = None;
        self.reset_icon_sys_fields();
    }

    pub(crate) fn folder_name(&self) -> String {
        let mut name = String::from(self.selected_prefix.as_str());
        name.push_str(&self.folder_base_name);
        name
    }

    pub(crate) fn psu_file_stem(&self) -> String {
        let mut stem = String::from(self.selected_prefix.as_str());
        stem.push_str(&self.psu_file_base_name);
        stem
    }

    pub(crate) fn default_output_file_name(&self) -> Option<String> {
        let stem = self.psu_file_stem();
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
            match self.default_output_file_name() {
                Some(file_name) => {
                    if let Some(parent) = Path::new(&self.output).parent() {
                        if !parent.as_os_str().is_empty() {
                            self.output = parent.join(&file_name).display().to_string();
                            return;
                        }
                    }
                    self.output = file_name;
                }
                None => self.output.clear(),
            }
        }
    }

    pub(crate) fn metadata_inputs_changed(&mut self, previous_default_output: Option<String>) {
        self.update_output_if_matches_default(previous_default_output);
        self.refresh_psu_toml_editor();
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

    pub(crate) fn reload_project_files(&mut self) {
        if let Some(folder) = self.folder.clone() {
            load_text_file_into_editor(folder.as_path(), "psu.toml", &mut self.psu_toml_editor);
            load_text_file_into_editor(folder.as_path(), "title.cfg", &mut self.title_cfg_editor);
            self.psu_toml_sync_blocked = false;
        } else {
            self.clear_text_editors();
        }
    }

    fn clear_text_editors(&mut self) {
        self.psu_toml_editor.clear();
        self.title_cfg_editor.clear();
        self.psu_toml_sync_blocked = false;
    }

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
        let Some(folder) = self.folder.clone() else {
            self.set_error_message(format!(
                "Select a folder before creating {file_name} from the template."
            ));
            return;
        };

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
        match tab {
            EditorTab::PsuSettings => self.open_psu_settings_tab(),
            EditorTab::PsuToml => self.open_psu_toml_tab(),
            EditorTab::TitleCfg => self.open_title_cfg_tab(),
            EditorTab::IconSys => self.open_icon_sys_tab(),
            EditorTab::TimestampAuto => self.open_timestamp_auto_tab(),
        }
    }

    pub(crate) fn open_psu_settings_tab(&mut self) {
        self.editor_tab = EditorTab::PsuSettings;
    }

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

fn text_editor_ui(
    ui: &mut egui::Ui,
    file_name: &str,
    editing_enabled: bool,
    save_enabled: bool,
    editor: &mut TextFileEditor,
) -> bool {
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

    let mut save_clicked = false;
    if save_enabled {
        ui.horizontal(|ui| {
            let button_label = format!("Save {file_name}");
            if ui
                .add_enabled(editor.modified, egui::Button::new(button_label))
                .clicked()
            {
                save_clicked = true;
            }

            if editor.modified {
                ui.colored_label(egui::Color32::YELLOW, "Unsaved changes");
            }
        });
    } else if editing_enabled {
        if editor.modified {
            ui.colored_label(egui::Color32::YELLOW, "Unsaved changes");
        }
        ui.label(format!(
            "Select a folder to enable saving {file_name} to disk."
        ));
    } else {
        ui.label(format!(
            "Select a folder or open a PSU to edit {file_name}."
        ));
    }

    save_clicked
}

impl eframe::App for PackerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_pack_job();

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
                        ui.heading("Packing PSU…");
                        ui.add_space(8.0);
                        ui.add(
                            egui::ProgressBar::new(progress)
                                .desired_width(200.0)
                                .animate(true),
                        );
                    });
                });
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui::file_picker::file_menu(self, ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.editor_tab, EditorTab::PsuSettings, "PSU Settings");
                let psu_toml_label = if self.psu_toml_editor.modified {
                    "psu.toml*"
                } else {
                    "psu.toml"
                };
                ui.selectable_value(&mut self.editor_tab, EditorTab::PsuToml, psu_toml_label);
                let title_cfg_label = if self.title_cfg_editor.modified {
                    "title.cfg*"
                } else {
                    "title.cfg"
                };
                ui.selectable_value(&mut self.editor_tab, EditorTab::TitleCfg, title_cfg_label);
                ui.selectable_value(&mut self.editor_tab, EditorTab::IconSys, "icon.sys");
                let timestamp_label = if self.timestamp_rules_modified {
                    "Timestamp rules*"
                } else {
                    "Timestamp rules"
                };
                ui.selectable_value(
                    &mut self.editor_tab,
                    EditorTab::TimestampAuto,
                    timestamp_label,
                );
            });
            ui.separator();

            match self.editor_tab {
                EditorTab::PsuSettings => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui::file_picker::folder_section(self, ui);

                        let showing_psu = self.showing_loaded_psu();
                        if showing_psu {
                            ui.add_space(8.0);
                            ui::file_picker::loaded_psu_section(self, ui);
                        }

                        ui.add_space(8.0);
                        ui::pack_controls::metadata_section(self, ui);

                        if !showing_psu {
                            ui.add_space(8.0);
                            ui::pack_controls::file_filters_section(self, ui);
                        }

                        ui.add_space(8.0);
                        ui::pack_controls::output_section(self, ui);

                        ui.add_space(8.0);
                        ui::pack_controls::packaging_section(self, ui);
                    });
                }
                EditorTab::PsuToml => {
                    let editing_enabled = self.folder.is_some() || self.showing_loaded_psu();
                    let save_clicked = text_editor_ui(
                        ui,
                        "psu.toml",
                        editing_enabled,
                        self.folder.is_some(),
                        &mut self.psu_toml_editor,
                    );
                    if save_clicked {
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
                                self.set_error_message(format!("Failed to save psu.toml: {err}"));
                            }
                        }
                    }
                }
                EditorTab::TitleCfg => {
                    let editing_enabled = self.folder.is_some() || self.showing_loaded_psu();
                    let save_clicked = text_editor_ui(
                        ui,
                        "title.cfg",
                        editing_enabled,
                        self.folder.is_some(),
                        &mut self.title_cfg_editor,
                    );
                    if save_clicked {
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
                                self.set_error_message(format!("Failed to save title.cfg: {err}"));
                            }
                        }
                    }
                }
                EditorTab::IconSys => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui::icon_sys::icon_sys_editor(self, ui);
                    });
                }
                EditorTab::TimestampAuto => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui::timestamps::timestamp_rules_editor(self, ui);
                    });
                }
            }
        });

        ui::dialogs::exit_confirmation(self, ctx);
    }
}
