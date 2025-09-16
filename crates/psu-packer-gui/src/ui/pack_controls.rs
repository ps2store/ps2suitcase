use std::path::{Path, PathBuf};

use chrono::NaiveDateTime;
use eframe::egui;

use crate::{IconFlagSelection, PackerApp, TIMESTAMP_FORMAT};

pub(crate) fn metadata_section(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.group(|ui| {
        ui.heading("Metadata");
        ui.small("Edit PSU metadata before or after selecting a folder.");
        egui::Grid::new("metadata_grid")
            .num_columns(2)
            .spacing(egui::vec2(12.0, 6.0))
            .show(ui, |ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut app.name);
                ui.end_row();

                ui.label("Timestamp");
                ui.add(egui::TextEdit::singleline(&mut app.timestamp).hint_text(TIMESTAMP_FORMAT));
                ui.end_row();

                ui.label("Icon.sys");
                let checkbox = ui.checkbox(&mut app.icon_sys_enabled, "Generate icon.sys metadata");
                checkbox.on_hover_text("Automatically create or update icon.sys when packing.");
                ui.end_row();

                ui.label("Icon title");
                ui.add_enabled(
                    app.icon_sys_enabled,
                    egui::TextEdit::singleline(&mut app.icon_sys_title),
                );
                ui.end_row();

                ui.label("Icon type");
                ui.add_enabled_ui(app.icon_sys_enabled, |ui| {
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_source("icon_sys_flag_combo")
                            .selected_text(app.icon_flag_label())
                            .show_ui(ui, |ui| {
                                for (idx, (_, label)) in
                                    crate::ICON_SYS_FLAG_OPTIONS.iter().enumerate()
                                {
                                    ui.selectable_value(
                                        &mut app.icon_sys_flag_selection,
                                        IconFlagSelection::Preset(idx),
                                        *label,
                                    );
                                }
                                ui.selectable_value(
                                    &mut app.icon_sys_flag_selection,
                                    IconFlagSelection::Custom,
                                    "Custom…",
                                );
                            });

                        if matches!(app.icon_sys_flag_selection, IconFlagSelection::Custom) {
                            ui.add(
                                egui::DragValue::new(&mut app.icon_sys_custom_flag)
                                    .clamp_range(0.0..=u16::MAX as f64),
                            );
                            ui.label(format!("0x{:04X}", app.icon_sys_custom_flag));
                        }
                    });
                });
                ui.end_row();
            });
    });
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
                if include_add {
                    app.handle_add_file(ListKind::Include);
                }
                if include_remove {
                    app.handle_remove_file(ListKind::Include);
                }

                let (exclude_add, exclude_remove) = file_list_ui(
                    &mut columns[1],
                    ListKind::Exclude.label(),
                    &mut app.exclude_files,
                    &mut app.selected_exclude,
                );
                if exclude_add {
                    app.handle_add_file(ListKind::Exclude);
                }
                if exclude_remove {
                    app.handle_remove_file(ListKind::Exclude);
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
        if ui
            .button("Pack")
            .on_hover_text("Create the PSU archive using the settings above.")
            .clicked()
        {
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
                match psu_packer::pack_with_config(folder, &output_path, config) {
                    Ok(_) => {
                        app.status = format!("Packed to {}", output_path.display());
                        app.clear_error_message();
                    }
                    Err(err) => {
                        let message = app.format_pack_error(folder, &output_path, err);
                        app.set_error_message(message);
                    }
                }
            } else {
                app.set_error_message("Please select a folder");
            }
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

    pub(crate) fn handle_add_file(&mut self, kind: ListKind) {
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

    pub(crate) fn handle_remove_file(&mut self, kind: ListKind) {
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

    fn list_mut(&mut self, kind: ListKind) -> (&mut Vec<String>, &mut Option<usize>) {
        match kind {
            ListKind::Include => (&mut self.include_files, &mut self.selected_include),
            ListKind::Exclude => (&mut self.exclude_files, &mut self.selected_exclude),
        }
    }
}
