#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::NaiveDateTime;
use eframe::egui;
use std::path::PathBuf;

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

            ui.horizontal(|ui| {
                if ui.button("Add file").clicked() {
                    add_clicked = true;
                }

                if ui
                    .add_enabled(selected.is_some(), egui::Button::new("Remove file"))
                    .clicked()
                {
                    remove_clicked = true;
                }
            });
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
            self.status = "Please select a folder before adding files".to_string();
            return;
        };

        let Some(path) = rfd::FileDialog::new().set_directory(&folder).pick_file() else {
            return;
        };

        let Ok(relative) = path.strip_prefix(&folder) else {
            self.status = "Selected file must be in the selected folder".to_string();
            return;
        };

        if relative.components().count() != 1 {
            self.status = "Selected file must be in the selected folder".to_string();
            return;
        }

        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            self.status = "Invalid file name".to_string();
            return;
        };

        let name = name.to_string();
        let duplicate = match self.file_mode {
            FileMode::Include => self.include_files.iter().any(|entry| entry == &name),
            FileMode::Exclude => self.exclude_files.iter().any(|entry| entry == &name),
        };

        if duplicate {
            self.status = "File is already listed".to_string();
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
            if ui.button("Select folder").clicked() {
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
                            self.status.clear();
                            if include_present && exclude_present {
                                self.status = "Config contains both include and exclude lists; using include list"
                                    .to_string();
                            }
                        }
                        Err(err) => {
                            self.status = format!("Error loading config: {err}");
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
            if !self.name.is_empty() || self.folder.is_some() {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.name);
                });
                ui.horizontal(|ui| {
                    ui.label("Timestamp:");
                    ui.text_edit_singleline(&mut self.timestamp);
                });
                ui.horizontal(|ui| {
                    ui.label("File mode:");
                    ui.radio_value(&mut self.file_mode, FileMode::Include, "Include");
                    ui.radio_value(&mut self.file_mode, FileMode::Exclude, "Exclude");
                });
                self.file_list_ui(ui);
            }
            ui.horizontal(|ui| {
                ui.label("Output:");
                ui.text_edit_singleline(&mut self.output);
                if ui.button("Browse").clicked() {
                    if let Some(file) = rfd::FileDialog::new()
                        .set_file_name(&self.output)
                        .save_file()
                    {
                        self.output = file.display().to_string();
                    }
                }
            });
            if ui.button("Pack").clicked() {
                if let Some(folder) = &self.folder {
                    if self.name.trim().is_empty() {
                        self.status = "Please provide a PSU name".to_string();
                        return;
                    }

                    let config = match self.build_config() {
                        Ok(config) => config,
                        Err(err) => {
                            self.status = err;
                            return;
                        }
                    };

                    let output_path = PathBuf::from(&self.output);
                    match psu_packer::pack_with_config(folder, &output_path, config) {
                        Ok(_) => self.status = format!("Packed to {}", output_path.display()),
                        Err(e) => self.status = format!("Error: {e}"),
                    }
                } else {
                    self.status = "Please select a folder".to_string();
                }
            }
            if !self.status.is_empty() {
                ui.label(&self.status);
            }
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
