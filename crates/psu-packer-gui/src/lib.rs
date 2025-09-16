use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use chrono::NaiveDateTime;
use eframe::egui;

pub mod ui;

pub use ui::{dialogs, file_picker, pack_controls};

pub(crate) const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
pub(crate) const ICON_SYS_FLAG_OPTIONS: &[(u16, &str)] = &[
    (0, "PS2 Save File"),
    (1, "Software (PS2)"),
    (2, "Unrecognized (0x02)"),
    (3, "Software (Pocketstation)"),
    (4, "Settings (PS2)"),
    (5, "System Driver"),
];

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

pub struct PackerApp {
    pub(crate) folder: Option<PathBuf>,
    pub(crate) output: String,
    pub(crate) status: String,
    pub(crate) error_message: Option<String>,
    pub(crate) name: String,
    pub(crate) timestamp: Option<NaiveDateTime>,
    pub(crate) include_files: Vec<String>,
    pub(crate) exclude_files: Vec<String>,
    pub(crate) selected_include: Option<usize>,
    pub(crate) selected_exclude: Option<usize>,
    pub(crate) loaded_psu_path: Option<PathBuf>,
    pub(crate) loaded_psu_files: Vec<String>,
    pub(crate) show_exit_confirm: bool,
    pub(crate) source_present_last_frame: bool,
    pub(crate) icon_sys_enabled: bool,
    pub(crate) icon_sys_title: String,
    pub(crate) icon_sys_flag_selection: IconFlagSelection,
    pub(crate) icon_sys_custom_flag: u16,
    pack_job: Option<PackJob>,
    editor_tab: EditorTab,
    psu_toml_editor: TextFileEditor,
    title_cfg_editor: TextFileEditor,
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
        Self {
            folder: None,
            output: String::new(),
            status: String::new(),
            error_message: None,
            name: String::new(),
            timestamp: None,
            include_files: Vec::new(),
            exclude_files: Vec::new(),
            selected_include: None,
            selected_exclude: None,
            loaded_psu_path: None,
            loaded_psu_files: Vec::new(),
            show_exit_confirm: false,
            source_present_last_frame: false,
            icon_sys_enabled: false,
            icon_sys_title: String::new(),
            icon_sys_flag_selection: IconFlagSelection::Preset(0),
            icon_sys_custom_flag: ICON_SYS_FLAG_OPTIONS[0].0,
            pack_job: None,
            editor_tab: EditorTab::PsuSettings,
            psu_toml_editor: TextFileEditor::default(),
            title_cfg_editor: TextFileEditor::default(),
        }
    }
}

impl PackerApp {
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
        self.icon_sys_title.clear();
        self.icon_sys_flag_selection = IconFlagSelection::Preset(0);
        self.icon_sys_custom_flag = ICON_SYS_FLAG_OPTIONS[0].0;
    }

    pub(crate) fn reset_metadata_fields(&mut self) {
        self.name.clear();
        self.timestamp = None;
        self.include_files.clear();
        self.exclude_files.clear();
        self.selected_include = None;
        self.selected_exclude = None;
        self.reset_icon_sys_fields();
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
        } else {
            self.clear_text_editors();
        }
    }

    fn clear_text_editors(&mut self) {
        self.psu_toml_editor.clear();
        self.title_cfg_editor.clear();
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
    folder_selected: bool,
    editor: &mut TextFileEditor,
) -> bool {
    if !folder_selected {
        ui.label(format!("Select a folder to edit {file_name}."));
        return false;
    }

    if let Some(message) = &editor.load_error {
        ui.colored_label(egui::Color32::YELLOW, message);
        ui.add_space(8.0);
    }

    let response = egui::ScrollArea::vertical()
        .id_source(format!("{file_name}_editor_scroll"))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut editor.content)
                    .desired_rows(20)
                    .code_editor(),
            )
        })
        .inner;

    if response.changed() {
        editor.modified = true;
    }

    ui.add_space(8.0);

    let mut save_clicked = false;
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
                    let save_clicked = text_editor_ui(
                        ui,
                        "psu.toml",
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
                    let save_clicked = text_editor_ui(
                        ui,
                        "title.cfg",
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
            }
        });

        ui::dialogs::exit_confirmation(self, ctx);
    }
}
