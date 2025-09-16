#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::NaiveDateTime;
use eframe::egui;
use ps2_filetypes::{PSUEntryKind, PSU};
use std::path::{Path, PathBuf};

const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const ICON_SYS_FLAG_OPTIONS: &[(u16, &str)] = &[
    (0, "PS2 Save File"),
    (1, "Software (PS2)"),
    (2, "Unrecognized (0x02)"),
    (3, "Software (Pocketstation)"),
    (4, "Settings (PS2)"),
    (5, "System Driver"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum IconFlagSelection {
    Preset(usize),
    Custom,
}

struct PackerApp {
    folder: Option<PathBuf>,
    output: String,
    status: String,
    error_message: Option<String>,
    name: String,
    timestamp: String,
    include_files: Vec<String>,
    exclude_files: Vec<String>,
    selected_include: Option<usize>,
    selected_exclude: Option<usize>,
    loaded_psu_path: Option<PathBuf>,
    loaded_psu_files: Vec<String>,
    show_exit_confirm: bool,
    source_present_last_frame: bool,
    icon_sys_enabled: bool,
    icon_sys_title: String,
    icon_sys_flag_selection: IconFlagSelection,
    icon_sys_custom_flag: u16,
}

#[derive(Copy, Clone)]
enum ListKind {
    Include,
    Exclude,
}

impl ListKind {
    fn label(self) -> &'static str {
        match self {
            ListKind::Include => "Include files",
            ListKind::Exclude => "Exclude files",
        }
    }
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
            timestamp: String::new(),
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
        }
    }
}

impl PackerApp {
    fn list_mut(&mut self, kind: ListKind) -> (&mut Vec<String>, &mut Option<usize>) {
        match kind {
            ListKind::Include => (&mut self.include_files, &mut self.selected_include),
            ListKind::Exclude => (&mut self.exclude_files, &mut self.selected_exclude),
        }
    }

    fn set_error_message<M>(&mut self, message: M)
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

    fn clear_error_message(&mut self) {
        self.error_message = None;
    }

    fn reset_icon_sys_fields(&mut self) {
        self.icon_sys_enabled = false;
        self.icon_sys_title.clear();
        self.icon_sys_flag_selection = IconFlagSelection::Preset(0);
        self.icon_sys_custom_flag = ICON_SYS_FLAG_OPTIONS[0].0;
    }

    fn reset_metadata_fields(&mut self) {
        self.name.clear();
        self.timestamp.clear();
        self.include_files.clear();
        self.exclude_files.clear();
        self.selected_include = None;
        self.selected_exclude = None;
        self.reset_icon_sys_fields();
    }

    fn icon_flag_label(&self) -> String {
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

    fn selected_icon_flag_value(&self) -> Result<u16, String> {
        match self.icon_sys_flag_selection {
            IconFlagSelection::Preset(index) => ICON_SYS_FLAG_OPTIONS
                .get(index)
                .map(|(value, _)| *value)
                .ok_or_else(|| "Invalid icon.sys flag selection".to_string()),
            IconFlagSelection::Custom => Ok(self.icon_sys_custom_flag),
        }
    }

    fn missing_include_files(&self, folder: &Path) -> Vec<String> {
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

    fn handle_open_psu(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PSU", &["psu"])
            .pick_file()
        else {
            return;
        };

        let data = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) => {
                self.set_error_message(format!("Failed to read {}: {err}", path.display()));
                return;
            }
        };

        let parsed = match std::panic::catch_unwind(|| PSU::new(data)) {
            Ok(psu) => psu,
            Err(_) => {
                self.set_error_message(format!("Failed to parse PSU file {}", path.display()));
                return;
            }
        };

        let entries = parsed.entries();
        let mut root_name: Option<String> = None;
        let mut root_timestamp = None;
        let mut files = Vec::new();

        for entry in &entries {
            match entry.kind {
                PSUEntryKind::Directory => {
                    if entry.name != "." && entry.name != ".." && root_name.is_none() {
                        root_name = Some(entry.name.clone());
                        root_timestamp = Some(entry.created);
                    }
                }
                PSUEntryKind::File => files.push(entry.name.clone()),
            }
        }

        let Some(name) = root_name else {
            self.set_error_message(format!("{} does not contain PSU metadata", path.display()));
            return;
        };

        self.name = name;
        self.timestamp = root_timestamp
            .map(|ts| ts.format(TIMESTAMP_FORMAT).to_string())
            .unwrap_or_default();
        self.loaded_psu_files = files;
        self.loaded_psu_path = Some(path.clone());
        self.clear_error_message();
        self.status = format!("Loaded PSU from {}", path.display());
        self.folder = None;
        self.include_files.clear();
        self.exclude_files.clear();
        self.selected_include = None;
        self.selected_exclude = None;
        self.reset_icon_sys_fields();

        if self.output.trim().is_empty() {
            self.output = path.display().to_string();
        }
    }

    fn browse_output_destination(&mut self) {
        if let Some(file) = rfd::FileDialog::new()
            .set_file_name(&self.output)
            .save_file()
        {
            self.output = file.display().to_string();
        }
    }

    fn format_load_error(folder: &Path, err: psu_packer::Error) -> String {
        match err {
            psu_packer::Error::NameError => {
                "Configuration contains an invalid PSU name.".to_string()
            }
            psu_packer::Error::ConfigError(message) => {
                format!("The psu.toml file is invalid: {message}")
            }
            psu_packer::Error::IOError(io_err) => {
                let config_path = folder.join("psu.toml");
                match io_err.kind() {
                    std::io::ErrorKind::NotFound => format!(
                        "Could not find {}. Create a psu.toml file in the selected folder.",
                        config_path.display()
                    ),
                    _ => format!("Failed to read {}: {}", config_path.display(), io_err),
                }
            }
        }
    }

    fn format_pack_error(
        &self,
        folder: &Path,
        output_path: &Path,
        err: psu_packer::Error,
    ) -> String {
        match err {
            psu_packer::Error::NameError => {
                "PSU name can only contain letters, numbers, spaces, underscores, and hyphens."
                    .to_string()
            }
            psu_packer::Error::ConfigError(message) => {
                format!("Configuration error: {message}")
            }
            psu_packer::Error::IOError(io_err) => {
                let missing_files = self.missing_include_files(folder);
                if !missing_files.is_empty() {
                    let formatted = missing_files
                        .into_iter()
                        .map(|name| format!("• {name}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    return format!(
                        "The following files referenced in the configuration are missing from {}:\n{}",
                        folder.display(),
                        formatted
                    );
                }

                match io_err.kind() {
                    std::io::ErrorKind::NotFound => {
                        if let Some(parent) = output_path.parent() {
                            if !parent.exists() {
                                return format!(
                                    "Cannot write the PSU file because the destination folder {} does not exist.",
                                    parent.display()
                                );
                            }
                        }
                        format!("A required file or folder could not be found: {io_err}")
                    }
                    std::io::ErrorKind::PermissionDenied => {
                        format!("Permission denied while accessing the file system: {io_err}")
                    }
                    _ => format!("File system error: {io_err}"),
                }
            }
        }
    }

    fn file_list_ui(
        ui: &mut egui::Ui,
        label: &str,
        files: &mut Vec<String>,
        selected: &mut Option<usize>,
    ) -> (bool, bool) {
        let mut add_clicked = false;
        let mut remove_clicked = false;
        let has_selection = selected.is_some();

        ui.horizontal(|ui| {
            ui.label(label);
            ui.add_space(ui.spacing().item_spacing.x);

            if ui
                .add(egui::Button::new("➕").small())
                .on_hover_text("Add files from the selected folder to this list.")
                .clicked()
            {
                add_clicked = true;
            }

            if ui
                .add_enabled(has_selection, egui::Button::new("➖").small())
                .on_hover_text("Remove the selected file from this list.")
                .clicked()
            {
                remove_clicked = true;
            }
        });

        egui::ScrollArea::vertical()
            .max_height(150.0)
            .show(ui, |ui| {
                for (idx, file) in files.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let is_selected = Some(idx) == *selected;
                        if ui.selectable_label(is_selected, file).clicked() {
                            *selected = Some(idx);
                        }

                        ui.add_space(ui.spacing().item_spacing.x);

                        if ui
                            .small_button("✖")
                            .on_hover_text("Remove this file from the list.")
                            .clicked()
                        {
                            *selected = Some(idx);
                            remove_clicked = true;
                        }
                    });
                }
            });

        (add_clicked, remove_clicked)
    }

    fn build_config(&self) -> Result<psu_packer::Config, String> {
        let timestamp = self.timestamp.trim();
        let timestamp = if timestamp.is_empty() {
            None
        } else {
            Some(
                NaiveDateTime::parse_from_str(timestamp, TIMESTAMP_FORMAT)
                    .map_err(|e| format!("Invalid timestamp: {e}"))?,
            )
        };

        let include = if self.include_files.is_empty() {
            None
        } else {
            Some(self.include_files.clone())
        };

        let exclude = if self.exclude_files.is_empty() {
            None
        } else {
            Some(self.exclude_files.clone())
        };

        let icon_sys = if self.icon_sys_enabled {
            let title = self.icon_sys_title.trim();
            if title.is_empty() {
                return Err("Icon.sys title cannot be empty when enabled".to_string());
            }

            let flag_value = self.selected_icon_flag_value()?;

            Some(psu_packer::IconSysConfig {
                flags: psu_packer::IconSysFlags::new(flag_value),
                title: title.to_string(),
                background_transparency: None,
                background_colors: None,
                light_directions: None,
                light_colors: None,
                ambient_color: None,
            })
        } else {
            None
        };

        Ok(psu_packer::Config {
            name: self.name.clone(),
            timestamp,
            include,
            exclude,
            icon_sys,
        })
    }

    fn handle_add_file(&mut self, kind: ListKind) {
        let Some(folder) = self.folder.clone() else {
            self.set_error_message("Please select a folder before adding files");
            return;
        };

        let list_label = kind.label();
        let (files, selected) = self.list_mut(kind);

        let Some(paths) = rfd::FileDialog::new().set_directory(&folder).pick_files() else {
            return;
        };

        if paths.is_empty() {
            return;
        }

        let mut invalid_entries = Vec::new();
        let mut last_added = None;

        for path in paths {
            let Ok(relative) = path.strip_prefix(&folder) else {
                invalid_entries.push(format!(
                    "{} (must be in the selected folder)",
                    path.display()
                ));
                continue;
            };

            if relative.components().count() != 1 {
                invalid_entries.push(format!(
                    "{} (must be in the selected folder)",
                    path.display()
                ));
                continue;
            }

            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                invalid_entries.push(format!("{} (invalid file name)", path.display()));
                continue;
            };

            let name = name.to_string();

            if files.iter().any(|entry| entry == &name) {
                invalid_entries.push(format!("{name} (already listed)"));
                continue;
            }

            files.push(name);
            last_added = Some(files.len() - 1);
        }

        if let Some(index) = last_added {
            *selected = Some(index);
        }

        if invalid_entries.is_empty() {
            if last_added.is_some() {
                self.clear_error_message();
                self.status.clear();
            }
        } else {
            let message = format!("Some files could not be added to the {list_label} list");
            self.set_error_message((message, invalid_entries));
        }
    }

    fn handle_remove_file(&mut self, kind: ListKind) {
        let (files, selected) = self.list_mut(kind);
        if let Some(idx) = selected.take() {
            files.remove(idx);
            if files.is_empty() {
                *selected = None;
            } else if idx >= files.len() {
                *selected = Some(files.len() - 1);
            } else {
                *selected = Some(idx);
            }
        }
    }
}

impl eframe::App for PackerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let source_present = self.folder.is_some()
            || self.loaded_psu_path.is_some()
            || !self.loaded_psu_files.is_empty();

        if !source_present && self.source_present_last_frame {
            self.reset_metadata_fields();
        }

        self.source_present_last_frame = source_present;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save PSU As...").clicked() {
                        self.browse_output_destination();
                        ui.close_menu();
                    }

                    if ui.button("Open PSU...").clicked() {
                        self.handle_open_psu();
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Exit").clicked() {
                        self.show_exit_confirm = true;
                        ui.close_menu();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.group(|ui| {
                    ui.heading("Folder");
                    ui.small("Select the PSU project folder containing psu.toml.");
                    ui.horizontal(|ui| {
                        if ui
                            .button("Select folder")
                            .on_hover_text(
                                "Pick the source directory to load configuration values.",
                            )
                            .clicked()
                        {
                            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                                match psu_packer::load_config(&dir) {
                                    Ok(config) => {
                                        let psu_packer::Config {
                                            name,
                                            timestamp,
                                            include,
                                            exclude,
                                            icon_sys,
                                        } = config;

                                        self.output = format!("{}.psu", name);
                                        self.name = name;
                                        self.timestamp = timestamp
                                            .map(|t| t.format(TIMESTAMP_FORMAT).to_string())
                                            .unwrap_or_default();
                                        self.include_files = include.unwrap_or_default();
                                        self.exclude_files = exclude.unwrap_or_default();
                                        self.selected_include = None;
                                        self.selected_exclude = None;
                                        if let Some(icon_cfg) = icon_sys {
                                            let psu_packer::IconSysConfig { flags, title, .. } =
                                                icon_cfg;
                                            let flag_value = flags.value();
                                            self.icon_sys_enabled = true;
                                            self.icon_sys_title = title;
                                            self.icon_sys_custom_flag = flag_value;
                                            if let Some(index) = ICON_SYS_FLAG_OPTIONS
                                                .iter()
                                                .position(|(value, _)| *value == flag_value)
                                            {
                                                self.icon_sys_flag_selection =
                                                    IconFlagSelection::Preset(index);
                                            } else {
                                                self.icon_sys_flag_selection =
                                                    IconFlagSelection::Custom;
                                            }
                                        } else {
                                            self.reset_icon_sys_fields();
                                        }
                                        self.clear_error_message();
                                        self.status.clear();
                                    }
                                    Err(err) => {
                                        let message = PackerApp::format_load_error(&dir, err);
                                        self.set_error_message(message);
                                        self.output.clear();
                                        self.name.clear();
                                        self.timestamp.clear();
                                        self.include_files.clear();
                                        self.exclude_files.clear();
                                        self.selected_include = None;
                                        self.selected_exclude = None;
                                        self.reset_icon_sys_fields();
                                    }
                                }
                                self.loaded_psu_path = None;
                                self.loaded_psu_files.clear();
                                self.folder = Some(dir);
                            }
                        }
                    });

                    if let Some(folder) = &self.folder {
                        ui.label(format!("Folder: {}", folder.display()));
                    }
                });

                let showing_psu = self.folder.is_none()
                    && (self.loaded_psu_path.is_some() || !self.loaded_psu_files.is_empty());

                if showing_psu {
                    ui.add_space(8.0);
                    ui.group(|ui| {
                        ui.heading("Loaded PSU");
                        ui.small("Review the files discovered in the opened PSU archive.");
                        if let Some(path) = &self.loaded_psu_path {
                            ui.label(format!("File: {}", path.display()));
                        }
                        egui::ScrollArea::vertical()
                            .max_height(150.0)
                            .show(ui, |ui| {
                                if self.loaded_psu_files.is_empty() {
                                    ui.label("The archive does not contain any files.");
                                } else {
                                    for file in &self.loaded_psu_files {
                                        ui.label(file);
                                    }
                                }
                            });
                    });
                }

                ui.add_space(8.0);
                ui.group(|ui| {
                    ui.heading("Metadata");
                    ui.small("Edit PSU metadata before or after selecting a folder.");
                    egui::Grid::new("metadata_grid")
                        .num_columns(2)
                        .spacing(egui::vec2(12.0, 6.0))
                        .show(ui, |ui| {
                            ui.label("Name");
                            ui.text_edit_singleline(&mut self.name);
                            ui.end_row();

                            ui.label("Timestamp");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.timestamp)
                                    .hint_text(TIMESTAMP_FORMAT),
                            );
                            ui.end_row();

                            ui.label("Icon.sys");
                            let checkbox = ui
                                .checkbox(&mut self.icon_sys_enabled, "Generate icon.sys metadata");
                            checkbox.on_hover_text(
                                "Automatically create or update icon.sys when packing.",
                            );
                            ui.end_row();

                            ui.label("Icon title");
                            ui.add_enabled(
                                self.icon_sys_enabled,
                                egui::TextEdit::singleline(&mut self.icon_sys_title),
                            );
                            ui.end_row();

                            ui.label("Icon type");
                            ui.add_enabled_ui(self.icon_sys_enabled, |ui| {
                                ui.horizontal(|ui| {
                                    egui::ComboBox::from_id_source("icon_sys_flag_combo")
                                        .selected_text(self.icon_flag_label())
                                        .show_ui(ui, |ui| {
                                            for (idx, (_, label)) in
                                                ICON_SYS_FLAG_OPTIONS.iter().enumerate()
                                            {
                                                ui.selectable_value(
                                                    &mut self.icon_sys_flag_selection,
                                                    IconFlagSelection::Preset(idx),
                                                    *label,
                                                );
                                            }
                                            ui.selectable_value(
                                                &mut self.icon_sys_flag_selection,
                                                IconFlagSelection::Custom,
                                                "Custom…",
                                            );
                                        });

                                    if matches!(
                                        self.icon_sys_flag_selection,
                                        IconFlagSelection::Custom
                                    ) {
                                        ui.add(
                                            egui::DragValue::new(&mut self.icon_sys_custom_flag)
                                                .clamp_range(0.0..=u16::MAX as f64),
                                        );
                                        ui.label(format!("0x{:04X}", self.icon_sys_custom_flag));
                                    }
                                });
                            });
                            ui.end_row();
                        });
                });

                if !showing_psu {
                    ui.add_space(8.0);

                    ui.group(|ui| {
                        ui.heading("File filters");
                        ui.small(
                            "Manage which files to include or exclude before creating the archive.",
                        );
                        if self.folder.is_some() {
                            ui.columns(2, |columns| {
                                let (include_add, include_remove) = Self::file_list_ui(
                                    &mut columns[0],
                                    ListKind::Include.label(),
                                    &mut self.include_files,
                                    &mut self.selected_include,
                                );
                                if include_add {
                                    self.handle_add_file(ListKind::Include);
                                }
                                if include_remove {
                                    self.handle_remove_file(ListKind::Include);
                                }

                                let (exclude_add, exclude_remove) = Self::file_list_ui(
                                    &mut columns[1],
                                    ListKind::Exclude.label(),
                                    &mut self.exclude_files,
                                    &mut self.selected_exclude,
                                );
                                if exclude_add {
                                    self.handle_add_file(ListKind::Exclude);
                                }
                                if exclude_remove {
                                    self.handle_remove_file(ListKind::Exclude);
                                }
                            });
                        } else {
                            ui.label("Select a folder to configure file filters.");
                        }
                    });
                }

                ui.add_space(8.0);

                ui.group(|ui| {
                    ui.heading("Output");
                    ui.small("Choose where the packed PSU file will be saved.");
                    egui::Grid::new("output_grid")
                        .num_columns(2)
                        .spacing(egui::vec2(12.0, 6.0))
                        .show(ui, |ui| {
                            ui.label("File path");
                            ui.text_edit_singleline(&mut self.output);
                            ui.end_row();

                            ui.label("");
                            if ui
                                .button("Browse")
                                .on_hover_text("Set a custom destination for the PSU file.")
                                .clicked()
                            {
                                self.browse_output_destination();
                            }
                            ui.end_row();
                        });
                });

                ui.add_space(8.0);

                ui.group(|ui| {
                    ui.heading("Packaging");
                    ui.small("Validate the configuration and generate the PSU archive.");
                    if ui
                        .button("Pack")
                        .on_hover_text("Create the PSU archive using the settings above.")
                        .clicked()
                    {
                        if let Some(folder) = &self.folder {
                            if self.name.trim().is_empty() {
                                self.set_error_message("Please provide a PSU name");
                                return;
                            }

                            let config = match self.build_config() {
                                Ok(config) => config,
                                Err(err) => {
                                    self.set_error_message(err);
                                    return;
                                }
                            };

                            let output_path = PathBuf::from(&self.output);
                            match psu_packer::pack_with_config(folder, &output_path, config) {
                                Ok(_) => {
                                    self.status = format!("Packed to {}", output_path.display());
                                    self.clear_error_message();
                                }
                                Err(err) => {
                                    let message = self.format_pack_error(folder, &output_path, err);
                                    self.set_error_message(message);
                                }
                            }
                        } else {
                            self.set_error_message("Please select a folder");
                        }
                    }

                    if let Some(error) = &self.error_message {
                        ui.colored_label(egui::Color32::RED, error);
                    }
                    if !self.status.is_empty() {
                        ui.label(&self.status);
                    }
                });
            });
        });

        if self.show_exit_confirm {
            egui::Window::new("Confirm Exit")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Are you sure you want to exit?");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let yes_clicked = ui.button("Yes").clicked();
                        let no_clicked = ui.button("No").clicked();

                        if yes_clicked {
                            self.show_exit_confirm = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        } else if no_clicked {
                            self.show_exit_confirm = false;
                        }
                    });
                });
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "PSU Packer",
        options,
        Box::new(|_cc| Box::<PackerApp>::default()),
    )
}
