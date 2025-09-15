#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::NaiveDateTime;
use eframe::egui;
use std::path::{Path, PathBuf};
use toml_edit::{value, DocumentMut};

const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

struct PackerApp {
    folder: Option<PathBuf>,
    output: String,
    status: String,
    name: String,
    timestamp: String,
}

impl Default for PackerApp {
    fn default() -> Self {
        Self {
            folder: None,
            output: String::new(),
            status: String::new(),
            name: String::new(),
            timestamp: String::new(),
        }
    }
}

impl PackerApp {
    fn save_config(&self, folder: &Path) -> Result<(), String> {
        let config_path = folder.join("psu.toml");
        let config_str = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read {}: {e}", config_path.display()))?;
        let mut document = config_str
            .parse::<DocumentMut>()
            .map_err(|e| format!("Failed to parse {}: {e}", config_path.display()))?;

        let Some(config_item) = document.get_mut("config") else {
            return Err("psu.toml is missing a [config] section".to_string());
        };
        let Some(config_table) = config_item.as_table_mut() else {
            return Err("psu.toml [config] section is not a table".to_string());
        };

        config_table.insert("name", value(self.name.clone()));

        let timestamp = self.timestamp.trim();
        if timestamp.is_empty() {
            config_table.remove("timestamp");
        } else {
            let parsed = NaiveDateTime::parse_from_str(timestamp, TIMESTAMP_FORMAT)
                .map_err(|e| format!("Invalid timestamp: {e}"))?;
            config_table.insert(
                "timestamp",
                value(parsed.format(TIMESTAMP_FORMAT).to_string()),
            );
        }

        std::fs::write(&config_path, document.to_string())
            .map_err(|e| format!("Failed to write {}: {e}", config_path.display()))?;

        Ok(())
    }
}

impl eframe::App for PackerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Select folder").clicked() {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    match psu_packer::load_config(&dir) {
                        Ok(config) => {
                            self.output = format!("{}.psu", config.name);
                            self.name = config.name;
                            self.timestamp = config
                                .timestamp
                                .map(|t| t.format(TIMESTAMP_FORMAT).to_string())
                                .unwrap_or_default();
                            self.status.clear();
                        }
                        Err(err) => {
                            self.status = format!("Error loading config: {err}");
                            self.output.clear();
                            self.name.clear();
                            self.timestamp.clear();
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

                    if let Err(err) = self.save_config(folder) {
                        self.status = err;
                        return;
                    }

                    let output_path = PathBuf::from(&self.output);
                    match psu_packer::pack_psu(folder, &output_path) {
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
