use std::path::{Path, PathBuf};

use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use eframe::egui;
use egui_extras::DatePickerButton;

use crate::{PackerApp, TIMESTAMP_FORMAT};

pub(crate) fn metadata_section(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.group(|ui| {
        ui.heading("Metadata");
        ui.small("Edit PSU metadata before or after selecting a folder.");
        egui::Grid::new("metadata_grid")
            .num_columns(2)
            .spacing(egui::vec2(12.0, 6.0))
            .show(ui, |ui| {
                ui.label("Name");
                if ui.text_edit_singleline(&mut app.name).changed() {
                    app.refresh_psu_toml_editor();
                }
                ui.end_row();

                ui.label("Timestamp");
                timestamp_picker_ui(app, ui);
                ui.end_row();

                ui.label("icon.sys");
                let mut label = "Configure icon.sys metadata in the dedicated tab.".to_string();
                if app.icon_sys_enabled {
                    if app.icon_sys_use_existing {
                        label.push_str(" Existing icon.sys file will be reused.");
                    } else {
                        label.push_str(" A new icon.sys will be generated.");
                    }
                }
                ui.small(label);
                ui.end_row();
            });

        if app.folder.is_some() && app.psu_toml_sync_blocked {
            ui.add_space(6.0);
            ui.colored_label(
                egui::Color32::YELLOW,
                "psu.toml has manual edits; automatic metadata syncing is paused.",
            );
        }
    });
}

fn timestamp_picker_ui(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        let previous_timestamp = app.timestamp;
        let default_timestamp = default_timestamp();
        let mut has_timestamp = app.timestamp.is_some();

        let mut new_timestamp = app.timestamp;

        if ui.checkbox(&mut has_timestamp, "Set timestamp").changed() {
            if has_timestamp {
                new_timestamp = Some(new_timestamp.unwrap_or(default_timestamp));
            } else {
                new_timestamp = None;
            }
        }

        if !has_timestamp {
            ui.small("No timestamp will be saved.");
            new_timestamp = None;
        } else {
            let mut timestamp = new_timestamp.unwrap_or(default_timestamp);
            let mut date: NaiveDate = timestamp.date();
            let time = timestamp.time();
            let mut hour = time.hour();
            let mut minute = time.minute();
            let mut second = time.second();
            let mut changed = false;

            ui.horizontal(|ui| {
                let date_response = ui.add(
                    DatePickerButton::new(&mut date).id_source("metadata_timestamp_date_picker"),
                );
                changed |= date_response.changed();

                ui.label("Time");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut hour)
                            .clamp_range(0..=23)
                            .suffix(" h"),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut minute)
                            .clamp_range(0..=59)
                            .suffix(" m"),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut second)
                            .clamp_range(0..=59)
                            .suffix(" s"),
                    )
                    .changed();
            });

            if changed {
                if let Some(new_time) = NaiveTime::from_hms_opt(hour, minute, second) {
                    timestamp = NaiveDateTime::new(date, new_time);
                }
            }

            new_timestamp = Some(timestamp);

            if let Some(ts) = new_timestamp {
                ui.small(format!("Selected: {}", ts.format(TIMESTAMP_FORMAT)));
            }
        }

        app.timestamp = new_timestamp;

        if app.timestamp != previous_timestamp {
            app.refresh_psu_toml_editor();
        }
    });
}

fn default_timestamp() -> NaiveDateTime {
    let now = Local::now().naive_local();
    now.with_nanosecond(0).unwrap_or(now)
}

pub(crate) fn file_filters_section(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.group(|ui| {
        ui.heading("File filters");
        ui.small("Manage which files to include or exclude before creating the archive.");
        if app.folder.is_some() {
            ui.columns(2, |columns| {
                let (include_add, include_remove) = file_list_ui(
                    &mut columns[0],
                    ListKind::Include.label(),
                    &mut app.include_files,
                    &mut app.selected_include,
                );
                if include_add && app.handle_add_file(ListKind::Include) {
                    app.refresh_psu_toml_editor();
                }
                if include_remove && app.handle_remove_file(ListKind::Include) {
                    app.refresh_psu_toml_editor();
                }

                let (exclude_add, exclude_remove) = file_list_ui(
                    &mut columns[1],
                    ListKind::Exclude.label(),
                    &mut app.exclude_files,
                    &mut app.selected_exclude,
                );
                if exclude_add && app.handle_add_file(ListKind::Exclude) {
                    app.refresh_psu_toml_editor();
                }
                if exclude_remove && app.handle_remove_file(ListKind::Exclude) {
                    app.refresh_psu_toml_editor();
                }
            });
        } else {
            ui.label("Select a folder to configure file filters.");
        }
    });
}

pub(crate) fn output_section(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.group(|ui| {
        ui.heading("Output");
        ui.small("Choose where the packed PSU file will be saved.");
        egui::Grid::new("output_grid")
            .num_columns(2)
            .spacing(egui::vec2(12.0, 6.0))
            .show(ui, |ui| {
                ui.label("File path");
                ui.text_edit_singleline(&mut app.output);
                ui.end_row();

                ui.label("");
                if ui
                    .button("Browse")
                    .on_hover_text("Set a custom destination for the PSU file.")
                    .clicked()
                {
                    app.browse_output_destination();
                }
                ui.end_row();
            });
    });
}

pub(crate) fn packaging_section(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.group(|ui| {
        ui.heading("Packaging");
        ui.small("Validate the configuration and generate the PSU archive.");
        let pack_in_progress = app.is_pack_running();
        let pack_button = ui
            .add_enabled(!pack_in_progress, egui::Button::new("Pack"))
            .on_hover_text("Create the PSU archive using the settings above.");

        if pack_button.clicked() {
            if let Some(folder) = &app.folder {
                if app.name.trim().is_empty() {
                    app.set_error_message("Please provide a PSU name");
                    return;
                }

                let config = match app.build_config() {
                    Ok(config) => config,
                    Err(err) => {
                        app.set_error_message(err);
                        return;
                    }
                };

                let output_path = PathBuf::from(&app.output);
                app.start_pack_job(folder.clone(), output_path, config);
            } else {
                app.set_error_message("Please select a folder");
            }
        }

        if pack_in_progress {
            ui.label("Packing in progress…");
        }

        if let Some(error) = &app.error_message {
            ui.colored_label(egui::Color32::RED, error);
        }
        if !app.status.is_empty() {
            ui.label(&app.status);
        }
    });
}

#[derive(Copy, Clone)]
pub(crate) enum ListKind {
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

impl PackerApp {
    pub(crate) fn browse_output_destination(&mut self) {
        if let Some(mut file) = rfd::FileDialog::new()
            .add_filter("PSU", &["psu"])
            .set_file_name(&self.output)
            .save_file()
        {
            let has_psu_extension = file
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("psu"))
                .unwrap_or(false);

            if !has_psu_extension {
                file.set_extension("psu");
            }

            self.output = file.display().to_string();
        }
    }

    pub(crate) fn build_config(&self) -> Result<psu_packer::Config, String> {
        self.validate_icon_sys_settings()?;
        self.config_from_state()
    }

    pub(crate) fn format_pack_error(
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

    pub(crate) fn handle_add_file(&mut self, kind: ListKind) -> bool {
        let Some(folder) = self.folder.clone() else {
            self.set_error_message("Please select a folder before adding files");
            return false;
        };

        let list_label = kind.label();
        let (files, selected) = self.list_mut(kind);

        let Some(paths) = rfd::FileDialog::new().set_directory(&folder).pick_files() else {
            return false;
        };

        if paths.is_empty() {
            return false;
        }

        let mut invalid_entries = Vec::new();
        let mut last_added = None;
        let mut added_any = false;

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
            added_any = true;
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

        added_any
    }

    pub(crate) fn handle_remove_file(&mut self, kind: ListKind) -> bool {
        let (files, selected) = self.list_mut(kind);
        let mut removed = false;
        if let Some(idx) = selected.take() {
            files.remove(idx);
            removed = true;
            if files.is_empty() {
                *selected = None;
            } else if idx >= files.len() {
                *selected = Some(files.len() - 1);
            } else {
                *selected = Some(idx);
            }
        }
        removed
    }

    fn list_mut(&mut self, kind: ListKind) -> (&mut Vec<String>, &mut Option<usize>) {
        match kind {
            ListKind::Include => (&mut self.include_files, &mut self.selected_include),
            ListKind::Exclude => (&mut self.exclude_files, &mut self.selected_exclude),
        }
    }

    fn validate_icon_sys_settings(&self) -> Result<(), String> {
        if self.icon_sys_enabled && !self.icon_sys_use_existing {
            let line1 = &self.icon_sys_title_line1;
            let line2 = &self.icon_sys_title_line2;

            if line1.chars().count() > 10 {
                return Err("Icon.sys line 1 cannot exceed 10 characters".to_string());
            }
            if line2.chars().count() > 10 {
                return Err("Icon.sys line 2 cannot exceed 10 characters".to_string());
            }
            let title_is_valid = |value: &str| {
                value
                    .chars()
                    .all(|c| c.is_ascii() && (!c.is_ascii_control() || c == ' '))
            };
            if !title_is_valid(line1) || !title_is_valid(line2) {
                return Err("Icon.sys titles only support printable ASCII characters".to_string());
            }

            let has_content = line1.chars().any(|c| !c.is_whitespace())
                || line2.chars().any(|c| !c.is_whitespace());
            if !has_content {
                return Err(
                    "Provide at least one non-space character for the icon.sys title".to_string(),
                );
            }

            self.selected_icon_flag_value()?;
        }

        Ok(())
    }

    fn config_from_state(&self) -> Result<psu_packer::Config, String> {
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

        let icon_sys = if self.icon_sys_enabled && !self.icon_sys_use_existing {
            let linebreak_pos = self.icon_sys_title_line1.chars().count() as u16;
            let combined_title =
                format!("{}{}", self.icon_sys_title_line1, self.icon_sys_title_line2);
            let flag_value = self.selected_icon_flag_value()?;

            Some(psu_packer::IconSysConfig {
                flags: psu_packer::IconSysFlags::new(flag_value),
                title: combined_title,
                linebreak_pos: Some(linebreak_pos),
                preset: self.icon_sys_selected_preset.clone(),
                background_transparency: Some(self.icon_sys_background_transparency),
                background_colors: Some(self.icon_sys_background_colors.to_vec()),
                light_directions: Some(self.icon_sys_light_directions.to_vec()),
                light_colors: Some(self.icon_sys_light_colors.to_vec()),
                ambient_color: Some(self.icon_sys_ambient_color),
            })
        } else {
            None
        };

        Ok(psu_packer::Config {
            name: self.name.clone(),
            timestamp: self.timestamp,
            include,
            exclude,
            icon_sys,
        })
    }

    pub(crate) fn refresh_psu_toml_editor(&mut self) {
        if self.folder.is_none() {
            self.psu_toml_sync_blocked = false;
            return;
        }

        if self.psu_toml_editor.modified {
            self.psu_toml_sync_blocked = true;
            return;
        }

        let config = match self.config_from_state() {
            Ok(config) => config,
            Err(_) => {
                self.psu_toml_sync_blocked = true;
                return;
            }
        };

        match config.to_toml_string() {
            Ok(serialized) => {
                self.psu_toml_editor.set_content(serialized);
                self.psu_toml_sync_blocked = false;
            }
            Err(_) => {
                self.psu_toml_sync_blocked = true;
            }
        }
    }
}
