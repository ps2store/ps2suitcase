#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::NaiveDateTime;
use eframe::egui;
use std::path::{Path, PathBuf};

const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

#[derive(Clone, Copy, PartialEq, Eq)]
enum FileMode {
    Include,
    Exclude,
}

impl Default for FileMode {
    fn default() -> Self {
        Self::Exclude
    }
}

struct PackerApp {
    folder: Option<PathBuf>,
    output: String,
    status: String,
    error_message: Option<String>,
    name: String,
    timestamp: String,
    file_mode: FileMode,
    include_files: Vec<String>,
    exclude_files: Vec<String>,
    selected_include: Option<usize>,
    selected_exclude: Option<usize>,
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
            file_mode: FileMode::default(),
            include_files: Vec::new(),
            exclude_files: Vec::new(),
            selected_include: None,
            selected_exclude: None,
        }
    }
}

impl PackerApp {
    fn current_list(&mut self) -> (&mut Vec<String>, &mut Option<usize>) {
        match self.file_mode {
            FileMode::Include => (&mut self.include_files, &mut self.selected_include),
            FileMode::Exclude => (&mut self.exclude_files, &mut self.selected_exclude),
        }
    }

    fn set_error_message<S: Into<String>>(&mut self, message: S) {
        self.error_message = Some(message.into());
        self.status.clear();
    }

    fn clear_error_message(&mut self) {
        self.error_message = None;
    }

    fn missing_include_files(&self, folder: &Path) -> Vec<String> {
        if !matches!(self.file_mode, FileMode::Include) {
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

    fn format_load_error(folder: &Path, err: psu_packer::Error) -> String {
        match err {
            psu_packer::Error::NameError => {
                "Configuration contains an invalid PSU name.".to_string()
            }
            psu_packer::Error::IncludeExcludeError => {
                "Configuration cannot define both include and exclude lists.".to_string()
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
            psu_packer::Error::IncludeExcludeError => {
                "Include and exclude lists cannot be used at the same time.".to_string()
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

    fn mode_label(&self) -> &'static str {
        match self.file_mode {
            FileMode::Include => "Included files",
            FileMode::Exclude => "Excluded files",
        }
    }

    fn file_list_ui(&mut self, ui: &mut egui::Ui) {
        let mode_label = self.mode_label();
        ui.label(mode_label);

        let mut add_clicked = false;
        let mut remove_clicked = false;

        {
            let (files, selected) = self.current_list();
            egui::ScrollArea::vertical()
                .max_height(150.0)
                .show(ui, |ui| {
                    for (idx, file) in files.iter().enumerate() {
                        let is_selected = Some(idx) == *selected;
                        if ui.selectable_label(is_selected, file).clicked() {
                            *selected = Some(idx);
                        }
                    }
                });
        }

        let selected_exists = {
            let (_, selected) = self.current_list();
            selected.is_some()
        };

        if ui.button("Add file").clicked() {
            add_clicked = true;
        }

        if ui
            .add_enabled(selected_exists, egui::Button::new("Remove file"))
            .clicked()
        {
            remove_clicked = true;
        }

        if add_clicked {
            self.handle_add_file();
        }

        if remove_clicked {
            self.handle_remove_file();
        }
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

        let (include, exclude) = match self.file_mode {
            FileMode::Include => (Some(self.include_files.clone()), None),
            FileMode::Exclude => (None, Some(self.exclude_files.clone())),
        };

        Ok(psu_packer::Config {
            name: self.name.clone(),
            timestamp,
            include,
            exclude,
        })
    }

    fn handle_add_file(&mut self) {
        let Some(folder) = self.folder.clone() else {
            self.set_error_message("Please select a folder before adding files");
            return;
        };

        let Some(path) = rfd::FileDialog::new().set_directory(&folder).pick_file() else {
            return;
        };

        let Ok(relative) = path.strip_prefix(&folder) else {
            self.set_error_message("Selected file must be in the selected folder");
            return;
        };

        if relative.components().count() != 1 {
            self.set_error_message("Selected file must be in the selected folder");
            return;
        }

        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            self.set_error_message("Invalid file name");
            return;
        };

        let name = name.to_string();
        let duplicate = match self.file_mode {
            FileMode::Include => self.include_files.iter().any(|entry| entry == &name),
            FileMode::Exclude => self.exclude_files.iter().any(|entry| entry == &name),
        };

        if duplicate {
            self.set_error_message("File is already listed");
            return;
        }

        match self.file_mode {
            FileMode::Include => {
                self.include_files.push(name);
                self.selected_include = Some(self.include_files.len() - 1);
            }
            FileMode::Exclude => {
                self.exclude_files.push(name);
                self.selected_exclude = Some(self.exclude_files.len() - 1);
            }
        }

        self.clear_error_message();
        self.status.clear();
    }

    fn handle_remove_file(&mut self) {
        let (files, selected) = self.current_list();
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
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.group(|ui| {
                ui.heading("Folder");
                ui.small("Select the PSU project folder containing psu.toml.");
                if ui
                    .button("Select folder")
                    .on_hover_text("Pick the source directory to load configuration values.")
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
                                } = config;

                                let include_present = include.is_some();
                                let exclude_present = exclude.is_some();

                                self.output = format!("{}.psu", name);
                                self.name = name;
                                self.timestamp = timestamp
                                    .map(|t| t.format(TIMESTAMP_FORMAT).to_string())
                                    .unwrap_or_default();
                                self.file_mode = if include_present {
                                    FileMode::Include
                                } else {
                                    FileMode::Exclude
                                };
                                self.include_files = include.unwrap_or_default();
                                self.exclude_files = exclude.unwrap_or_default();
                                self.selected_include = None;
                                self.selected_exclude = None;
                                self.clear_error_message();
                                self.status.clear();
                                if include_present && exclude_present {
                                    self.status = "Config contains both include and exclude lists; using include list"
                                        .to_string();
                                }
                            }
                            Err(err) => {
                                let message = PackerApp::format_load_error(&dir, err);
                                self.set_error_message(message);
                                self.output.clear();
                                self.name.clear();
                                self.timestamp.clear();
                                self.file_mode = FileMode::default();
                                self.include_files.clear();
                                self.exclude_files.clear();
                                self.selected_include = None;
                                self.selected_exclude = None;
                            }
                        }
                        self.folder = Some(dir);
                    }
                }

                if let Some(folder) = &self.folder {
                    ui.label(format!("Folder: {}", folder.display()));
                }
            });

            ui.add_space(8.0);

            ui.group(|ui| {
                ui.heading("Metadata");
                ui.small("Review or edit metadata loaded from the selected folder.");
                if self.folder.is_some() || !self.name.is_empty() {
                    egui::Grid::new("metadata_grid")
                        .num_columns(2)
                        .spacing(egui::vec2(12.0, 6.0))
                        .show(ui, |ui| {
                            ui.label("Name");
                            ui.text_edit_singleline(&mut self.name);
                            ui.end_row();

                            ui.label("Timestamp");
                            ui.text_edit_singleline(&mut self.timestamp);
                            ui.end_row();

                            ui.label("File mode");
                            ui.vertical(|ui| {
                                ui.radio_value(&mut self.file_mode, FileMode::Include, "Include");
                                ui.radio_value(&mut self.file_mode, FileMode::Exclude, "Exclude");
                            });
                            ui.end_row();
                        });
                } else {
                    ui.label("Select a folder to load metadata options.");
                }
            });

            ui.add_space(8.0);

            ui.group(|ui| {
                ui.heading("File filters");
                ui.small("Manage include or exclude lists before creating the archive.");
                if self.folder.is_some() || !self.name.is_empty() {
                    self.file_list_ui(ui);
                } else {
                    ui.label("Select a folder to configure file filters.");
                }
            });

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
                            if let Some(file) = rfd::FileDialog::new()
                                .set_file_name(&self.output)
                                .save_file()
                            {
                                self.output = file.display().to_string();
                            }
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
