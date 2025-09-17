use std::{fs, path::Path};

use eframe::egui;
use ps2_filetypes::{IconSys, PSUEntryKind, PSU};

use crate::{sas_timestamps, PackerApp, SasPrefix};

pub(crate) fn file_menu(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.menu_button("File", |ui| {
        if ui.button("Save PSU As...").clicked() {
            app.browse_output_destination();
            ui.close_menu();
        }

        if ui.button("Open PSU...").clicked() {
            app.handle_open_psu();
            ui.close_menu();
        }

        ui.add_enabled_ui(app.folder.is_some(), |ui| {
            if ui.button("Edit psu.toml").clicked() {
                app.open_psu_toml_tab();
                ui.close_menu();
            }

            if ui.button("Edit title.cfg").clicked() {
                app.open_title_cfg_tab();
                ui.close_menu();
            }

            if ui.button("Edit icon.sys").clicked() {
                app.open_icon_sys_tab();
                ui.close_menu();
            }

            if ui.button("Create psu.toml from template").clicked() {
                app.create_psu_toml_from_template();
                ui.close_menu();
            }

            if ui.button("Create title.cfg from template").clicked() {
                app.create_title_cfg_from_template();
                ui.close_menu();
            }
        });

        ui.separator();

        if ui.button("Exit").clicked() {
            app.show_exit_confirm = true;
            ui.close_menu();
        }
    });
}

pub(crate) fn folder_section(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.group(|ui| {
        ui.heading("Folder");
        ui.small("Select the PSU project folder containing psu.toml.");
        ui.horizontal(|ui| {
            let spacing = ui.spacing().item_spacing.x;
            ui.spacing_mut().item_spacing.x = spacing.max(8.0);

            if ui
                .button("Select folder")
                .on_hover_text("Pick the source directory to load configuration values.")
                .clicked()
            {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    let mut deterministic_timestamp_added = false;
                    match psu_packer::load_config(&folder) {
                        Ok(config) => {
                            let psu_packer::Config {
                                name,
                                timestamp,
                                include,
                                exclude,
                                icon_sys,
                            } = config;

                            app.set_folder_name_from_full(&name);
                            app.psu_file_base_name = app.folder_base_name.clone();
                            app.output = app.default_output_file_name().unwrap_or_default();
                            let planned_timestamp = timestamp
                                .or_else(|| sas_timestamps::planned_timestamp_for_folder(&folder));
                            deterministic_timestamp_added =
                                timestamp.is_none() && planned_timestamp.is_some();
                            app.timestamp = planned_timestamp;
                            app.include_files = include.unwrap_or_default();
                            app.exclude_files = exclude.unwrap_or_default();
                            app.selected_include = None;
                            app.selected_exclude = None;
                            app.clear_error_message();
                            app.status.clear();

                            let icon_sys_path = folder.join("icon.sys");
                            let mut parsed_icon_sys = None;
                            if icon_sys_path.is_file() {
                                match fs::read(&icon_sys_path) {
                                    Ok(bytes) => {
                                        match std::panic::catch_unwind(|| IconSys::new(bytes)) {
                                            Ok(icon_sys) => parsed_icon_sys = Some(icon_sys),
                                            Err(_) => {
                                                app.set_error_message(format!(
                                                    "Failed to parse {} as an icon.sys file.",
                                                    icon_sys_path.display()
                                                ));
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        app.set_error_message(format!(
                                            "Failed to read {}: {}",
                                            icon_sys_path.display(),
                                            err
                                        ));
                                    }
                                }
                            }

                            if let Some(icon_cfg) = icon_sys {
                                app.apply_icon_sys_config(icon_cfg, parsed_icon_sys.as_ref());
                            } else if let Some(existing_icon_sys) = parsed_icon_sys.as_ref() {
                                app.apply_icon_sys_file(existing_icon_sys);
                            } else {
                                app.reset_icon_sys_fields();
                            }

                            app.icon_sys_existing = parsed_icon_sys;
                        }
                        Err(err) => {
                            let message = format_load_error(&folder, err);
                            app.set_error_message(message);
                            app.output.clear();
                            app.selected_prefix = SasPrefix::default();
                            app.folder_base_name.clear();
                            app.psu_file_base_name.clear();
                            app.timestamp = None;
                            app.include_files.clear();
                            app.exclude_files.clear();
                            app.selected_include = None;
                            app.selected_exclude = None;
                            app.reset_icon_sys_fields();
                        }
                    }
                    app.loaded_psu_path = None;
                    app.loaded_psu_files.clear();
                    app.folder = Some(folder.clone());
                    app.reload_project_files();
                    if deterministic_timestamp_added {
                        app.refresh_psu_toml_editor();
                    }
                    if app.icon_sys_enabled {
                        app.open_icon_sys_tab();
                    } else {
                        app.open_psu_settings_tab();
                    }
                }
            }

            if ui
                .button("Load PSU...")
                .on_hover_text(
                    "Open an existing PSU archive to populate the editor from its metadata.",
                )
                .clicked()
            {
                app.handle_open_psu();
            }
        });

        if let Some(folder) = &app.folder {
            ui.label(format!("Folder: {}", folder.display()));
        }
    });
}

pub(crate) fn loaded_psu_section(app: &PackerApp, ui: &mut egui::Ui) {
    ui.group(|ui| {
        ui.heading("Loaded PSU");
        ui.small("Review the files discovered in the opened PSU archive.");
        if let Some(path) = &app.loaded_psu_path {
            ui.label(format!("File: {}", path.display()));
        }
        egui::ScrollArea::vertical()
            .max_height(150.0)
            .show(ui, |ui| {
                if app.loaded_psu_files.is_empty() {
                    ui.label("The archive does not contain any files.");
                } else {
                    for file in &app.loaded_psu_files {
                        ui.label(file);
                    }
                }
            });
    });
}

impl PackerApp {
    pub(crate) fn handle_open_psu(&mut self) {
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
        let mut psu_toml_bytes: Option<Vec<u8>> = None;
        let mut title_cfg_bytes: Option<Vec<u8>> = None;
        let mut icon_sys_bytes: Option<Vec<u8>> = None;

        for entry in &entries {
            match entry.kind {
                PSUEntryKind::Directory => {
                    if entry.name != "." && entry.name != ".." && root_name.is_none() {
                        root_name = Some(entry.name.clone());
                        root_timestamp = Some(entry.created);
                    }
                }
                PSUEntryKind::File => {
                    let name_matches = entry.name.as_str();
                    if psu_toml_bytes.is_none() && name_matches.eq_ignore_ascii_case("psu.toml") {
                        if let Some(bytes) = entry.contents.clone() {
                            psu_toml_bytes = Some(bytes);
                        }
                    }
                    if title_cfg_bytes.is_none() && name_matches.eq_ignore_ascii_case("title.cfg") {
                        if let Some(bytes) = entry.contents.clone() {
                            title_cfg_bytes = Some(bytes);
                        }
                    }
                    if icon_sys_bytes.is_none() && name_matches.eq_ignore_ascii_case("icon.sys") {
                        if let Some(bytes) = entry.contents.clone() {
                            icon_sys_bytes = Some(bytes);
                        }
                    }
                    files.push(entry.name.clone());
                }
            }
        }

        let Some(name) = root_name else {
            self.set_error_message(format!("{} does not contain PSU metadata", path.display()));
            return;
        };

        self.set_folder_name_from_full(&name);
        if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
            self.set_psu_file_base_from_full(stem);
        } else {
            self.psu_file_base_name = self.folder_base_name.clone();
        }
        self.timestamp = root_timestamp;
        self.loaded_psu_files = files;
        self.loaded_psu_path = Some(path.clone());
        self.clear_error_message();
        self.status = format!("Loaded PSU from {}", path.display());
        self.folder = None;
        self.include_files.clear();
        self.exclude_files.clear();
        self.selected_include = None;
        self.selected_exclude = None;
        let decode_text = |bytes: Vec<u8>| match String::from_utf8(bytes) {
            Ok(content) => content,
            Err(err) => {
                let bytes = err.into_bytes();
                String::from_utf8_lossy(&bytes).into_owned()
            }
        };

        if let Some(bytes) = psu_toml_bytes {
            self.psu_toml_editor.set_content(decode_text(bytes));
        } else {
            self.psu_toml_editor
                .set_error_message("psu.toml not found in the opened archive.".to_string());
        }

        if let Some(bytes) = title_cfg_bytes {
            self.title_cfg_editor.set_content(decode_text(bytes));
        } else {
            self.title_cfg_editor
                .set_error_message("title.cfg not found in the opened archive.".to_string());
        }

        self.psu_toml_sync_blocked = false;

        if let Some(bytes) = icon_sys_bytes {
            match std::panic::catch_unwind(|| IconSys::new(bytes)) {
                Ok(parsed_icon_sys) => {
                    self.apply_icon_sys_file(&parsed_icon_sys);
                }
                Err(_) => {
                    self.reset_icon_sys_fields();
                    self.set_error_message(format!(
                        "Failed to parse icon.sys from {}.",
                        path.display()
                    ));
                }
            }
        } else {
            self.reset_icon_sys_fields();
        }
        self.open_psu_settings_tab();

        if self.output.trim().is_empty() {
            self.output = path.display().to_string();
        }
    }
}

fn format_load_error(folder: &Path, err: psu_packer::Error) -> String {
    match err {
        psu_packer::Error::NameError => "Configuration contains an invalid PSU name.".to_string(),
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
