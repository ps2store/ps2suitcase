use std::path::Path;

use eframe::egui;

use crate::{ui::theme, PackerApp, SasPrefix, ICON_SYS_TITLE_CHAR_LIMIT};
use ps2_filetypes::sjis;

pub(crate) fn metadata_section(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.set_width(ui.available_width());
    ui.group(|ui| {
        ui.heading(theme::display_heading_text(ui, "Metadata"));
        ui.small("Edit PSU metadata before or after selecting a folder.");
        let previous_default_output = app.default_output_file_name();
        let mut metadata_changed = false;

        egui::Grid::new("metadata_grid")
            .num_columns(2)
            .spacing(egui::vec2(12.0, 6.0))
            .show(ui, |ui| {
                ui.label("SAS prefix");
                let prefix_changed = egui::ComboBox::from_id_source("metadata_prefix_combo")
                    .selected_text(app.selected_prefix.label())
                    .show_ui(ui, |ui| {
                        let mut changed = false;
                        for prefix in SasPrefix::iter_all() {
                            let response = ui.selectable_value(
                                &mut app.selected_prefix,
                                prefix,
                                prefix.label(),
                            );
                            if response.changed() {
                                changed = true;
                            }
                        }
                        changed
                    })
                    .inner
                    .unwrap_or(false);
                if prefix_changed {
                    metadata_changed = true;
                }
                ui.end_row();

                ui.label("Folder name");
                if ui.text_edit_singleline(&mut app.folder_base_name).changed() {
                    metadata_changed = true;
                }
                ui.end_row();

                ui.label("PSU filename");
                if ui
                    .text_edit_singleline(&mut app.psu_file_base_name)
                    .changed()
                {
                    metadata_changed = true;
                }
                ui.end_row();

                ui.label("Timestamp");
                crate::ui::timestamps::metadata_timestamp_section(app, ui);
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

        if metadata_changed {
            app.metadata_inputs_changed(previous_default_output);
        }

        #[cfg(feature = "psu-toml-editor")]
        if app.folder.is_some() && app.psu_toml_sync_blocked {
            ui.add_space(6.0);
            ui.colored_label(
                egui::Color32::YELLOW,
                "psu.toml has manual edits; automatic metadata syncing is paused.",
            );
        }
    });
}

pub(crate) fn file_filters_section(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.set_width(ui.available_width());
    ui.group(|ui| {
        ui.heading(theme::display_heading_text(ui, "File filters"));
        ui.small("Manage which files to include or exclude before creating the archive.");
        let folder_selected = app.folder.is_some();
        if !folder_selected {
            ui.small("No folder selected. Enter file names manually or choose a folder to browse.");
        }
        ui.columns(2, |columns| {
            let include_actions = file_list_ui(
                &mut columns[0],
                ListKind::Include.label(),
                &mut app.include_files,
                &mut app.selected_include,
                &mut app.include_manual_entry,
                folder_selected,
            );
            if include_actions.browse_add && app.handle_add_file(ListKind::Include) {
                app.refresh_psu_toml_editor();
            }
            if let Some(entry) = include_actions.manual_add {
                if app.handle_add_file_from_entry(ListKind::Include, &entry) {
                    app.refresh_psu_toml_editor();
                }
            }
            if include_actions.remove && app.handle_remove_file(ListKind::Include) {
                app.refresh_psu_toml_editor();
            }

            let exclude_actions = file_list_ui(
                &mut columns[1],
                ListKind::Exclude.label(),
                &mut app.exclude_files,
                &mut app.selected_exclude,
                &mut app.exclude_manual_entry,
                folder_selected,
            );
            if exclude_actions.browse_add && app.handle_add_file(ListKind::Exclude) {
                app.refresh_psu_toml_editor();
            }
            if let Some(entry) = exclude_actions.manual_add {
                if app.handle_add_file_from_entry(ListKind::Exclude, &entry) {
                    app.refresh_psu_toml_editor();
                }
            }
            if exclude_actions.remove && app.handle_remove_file(ListKind::Exclude) {
                app.refresh_psu_toml_editor();
            }
        });
    });
}

pub(crate) fn output_section(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.set_width(ui.available_width());
    ui.group(|ui| {
        ui.heading(theme::display_heading_text(ui, "Output"));
        ui.small("Choose where the packed PSU file will be saved.");
        egui::Grid::new("output_grid")
            .num_columns(2)
            .spacing(egui::vec2(12.0, 6.0))
            .show(ui, |ui| {
                ui.label("Packed PSU path");
                let trimmed_output = app.output.trim();
                if trimmed_output.is_empty() {
                    ui.weak("No destination selected");
                } else {
                    ui.label(egui::RichText::new(trimmed_output).monospace());
                }
                ui.end_row();

                ui.label("");
                if ui
                    .button("Choose destination")
                    .on_hover_text("Pick where the PSU file will be created or updated.")
                    .clicked()
                {
                    app.browse_output_destination();
                }
                ui.end_row();
            });
    });
}

pub(crate) fn packaging_section(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.set_width(ui.available_width());
    ui.group(|ui| {
        ui.heading(theme::display_heading_text(ui, "Packaging"));
        ui.small("Validate the configuration and generate the PSU archive.");
        let pack_in_progress = app.is_pack_running();
        if !app.missing_required_project_files.is_empty() {
            let warning = PackerApp::format_missing_required_files_message(
                &app.missing_required_project_files,
            );
            ui.colored_label(egui::Color32::YELLOW, warning);
        }
        ui.horizontal_wrapped(|ui| {
            let pack_button = ui
                .add_enabled(!pack_in_progress, egui::Button::new("Pack PSU"))
                .on_hover_text("Create the PSU archive using the settings above.");

            if pack_button.clicked() {
                app.handle_pack_request();
            }

            let update_button = ui
                .add_enabled(!pack_in_progress, egui::Button::new("Update PSU"))
                .on_hover_text("Repack the current project into the existing PSU file.");
            if update_button.clicked() {
                app.handle_update_psu_request();
            }

            let export_button = ui
                .add_enabled(
                    !pack_in_progress,
                    egui::Button::new("Save as Folder with contents"),
                )
                .on_hover_text("Export the contents of the current PSU archive to a folder.");
            if export_button.clicked() {
                app.handle_save_as_folder_with_contents();
            }
        });

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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use ps2_filetypes::sjis;
    use std::path::PathBuf;

    #[test]
    fn config_from_state_appends_psu_toml_once() {
        let mut app = PackerApp::default();
        app.folder_base_name = "SAVE".to_string();
        app.psu_file_base_name = "SAVE".to_string();
        app.selected_prefix = SasPrefix::App;

        let config = app.config_from_state().expect("configuration should build");
        assert_eq!(config.exclude, Some(vec!["psu.toml".to_string()]));
        assert!(
            app.exclude_files.is_empty(),
            "building the configuration should not modify the exclude list"
        );

        app.exclude_files = vec!["DATA.BIN".to_string()];
        let config_with_manual_entry = app
            .config_from_state()
            .expect("configuration should include manual exclude");
        assert_eq!(
            config_with_manual_entry.exclude,
            Some(vec!["DATA.BIN".to_string(), "psu.toml".to_string()])
        );

        app.exclude_files = vec!["psu.toml".to_string()];
        let config_with_duplicate = app
            .config_from_state()
            .expect("configuration should handle duplicate entries");
        assert_eq!(
            config_with_duplicate.exclude,
            Some(vec!["psu.toml".to_string()])
        );
    }

    #[test]
    fn build_config_uses_loaded_psu_edits() {
        let mut app = PackerApp::default();
        app.loaded_psu_path = Some(PathBuf::from("input.psu"));
        app.selected_prefix = SasPrefix::Emu;
        app.folder_base_name = "SAVE".to_string();
        let timestamp = NaiveDate::from_ymd_opt(2023, 11, 14)
            .and_then(|date| date.and_hms_opt(12, 34, 56))
            .expect("valid timestamp");
        app.timestamp = Some(timestamp);
        app.include_files.push("FILE.BIN".to_string());
        app.exclude_files.push("SKIP.DAT".to_string());

        let config = app.build_config().expect("config builds successfully");
        assert_eq!(config.name, "EMU_SAVE");
        assert_eq!(config.timestamp, Some(timestamp));
        assert_eq!(config.include, Some(vec!["FILE.BIN".to_string()]));
        assert_eq!(
            config.exclude,
            Some(vec!["SKIP.DAT".to_string(), "psu.toml".to_string()])
        );
    }

    #[test]
    fn manual_filter_entries_allowed_without_folder() {
        let mut app = PackerApp::default();
        app.selected_prefix = SasPrefix::App;
        app.folder_base_name = "SAVE".to_string();

        assert!(app.handle_add_file_from_entry(ListKind::Include, "BOOT.ELF"));
        assert!(app.handle_add_file_from_entry(ListKind::Exclude, "THUMBS.DB"));

        let config = app.build_config().expect("config builds successfully");
        assert_eq!(config.include, Some(vec!["BOOT.ELF".to_string()]));
        assert_eq!(
            config.exclude,
            Some(vec!["THUMBS.DB".to_string(), "psu.toml".to_string()])
        );
    }

    #[test]
    fn manual_filter_entries_trim_and_reject_duplicates() {
        let mut app = PackerApp::default();

        assert!(app.handle_add_file_from_entry(ListKind::Include, "  DATA.BIN  "));
        assert_eq!(app.include_files, vec!["DATA.BIN"]);

        assert!(!app.handle_add_file_from_entry(ListKind::Include, "DATA.BIN"));
        assert_eq!(app.include_files, vec!["DATA.BIN"]);
        assert!(app.error_message.is_some());
    }

    #[test]
    fn config_from_state_uses_shift_jis_byte_linebreaks() {
        let mut app = PackerApp::default();
        app.selected_prefix = SasPrefix::App;
        app.folder_base_name = "SAVE".to_string();
        app.psu_file_base_name = "SAVE".to_string();
        app.icon_sys_enabled = true;
        app.icon_sys_use_existing = false;
        app.icon_sys_title_line1 = "メモ".to_string();
        app.icon_sys_title_line2 = "リーカード".to_string();

        let config = app.config_from_state().expect("configuration should build");
        let icon_sys = config.icon_sys.expect("icon_sys configuration present");
        let expected_break = sjis::encode_sjis(&app.icon_sys_title_line1).unwrap().len() as u16;

        assert_eq!(icon_sys.linebreak_pos, Some(expected_break));
    }
}

struct FileListActions {
    browse_add: bool,
    remove: bool,
    manual_add: Option<String>,
}

fn file_list_ui(
    ui: &mut egui::Ui,
    label: &str,
    files: &mut Vec<String>,
    selected: &mut Option<usize>,
    manual_entry: &mut String,
    allow_browse: bool,
) -> FileListActions {
    let mut browse_clicked = false;
    let mut remove_clicked = false;
    let mut manual_added: Option<String> = None;
    let has_selection = selected.is_some();

    ui.horizontal(|ui| {
        ui.label(label);
        ui.add_space(ui.spacing().item_spacing.x);

        let browse_button = egui::Button::new("📁").small();
        let browse_response = ui
            .add_enabled(allow_browse, browse_button)
            .on_hover_text("Browse for files in the selected folder.");
        if browse_response.clicked() {
            browse_clicked = true;
        }

        if ui
            .add_enabled(has_selection, egui::Button::new("➖").small())
            .on_hover_text("Remove the selected file from this list.")
            .clicked()
        {
            remove_clicked = true;
        }
    });

    ui.horizontal(|ui| {
        let response =
            ui.add(egui::TextEdit::singleline(manual_entry).hint_text("Add file by name"));
        let add_manual = ui
            .add(egui::Button::new("Add").small())
            .on_hover_text("Add the typed entry to this list.")
            .clicked();
        let enter_pressed = ui.input(|input| input.key_pressed(egui::Key::Enter));

        if add_manual || (response.lost_focus() && enter_pressed) {
            let value = manual_entry.trim();
            if !value.is_empty() {
                manual_added = Some(value.to_string());
                manual_entry.clear();
            }
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

    FileListActions {
        browse_add: browse_clicked,
        remove: remove_clicked,
        manual_add: manual_added,
    }
}

impl PackerApp {
    pub(crate) fn browse_output_destination(&mut self) -> bool {
        let mut dialog = rfd::FileDialog::new().add_filter("PSU", &["psu"]);

        let trimmed_output = self.output.trim();
        if trimmed_output.is_empty() {
            if let Some(default_dir) = self.default_output_directory(None) {
                dialog = dialog.set_directory(default_dir);
            }
            if let Some(default_name) = self.default_output_file_name() {
                dialog = dialog.set_file_name(&default_name);
            }
        } else {
            let current_path = Path::new(trimmed_output);
            if let Some(parent) = current_path.parent() {
                if !parent.as_os_str().is_empty() {
                    dialog = dialog.set_directory(parent);
                } else if let Some(default_dir) = self.default_output_directory(None) {
                    dialog = dialog.set_directory(default_dir);
                }
            } else if let Some(default_dir) = self.default_output_directory(None) {
                dialog = dialog.set_directory(default_dir);
            }

            if let Some(existing_name) = current_path.file_name().and_then(|name| name.to_str()) {
                dialog = dialog.set_file_name(existing_name);
            } else if let Some(default_name) = self.default_output_file_name() {
                dialog = dialog.set_file_name(&default_name);
            }
        }

        if let Some(mut file) = dialog.save_file() {
            let has_psu_extension = file
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("psu"))
                .unwrap_or(false);

            if !has_psu_extension {
                file.set_extension("psu");
            }

            self.output = file.display().to_string();
            true
        } else {
            false
        }
    }

    pub(crate) fn ensure_output_destination_selected(&mut self) -> bool {
        if self.output.trim().is_empty() {
            if let Some(path) = self.default_output_path() {
                self.output = path.display().to_string();
            }
        }

        if self.output.trim().is_empty() {
            return self.browse_output_destination();
        }

        true
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
            return false;
        };

        let list_label = kind.label();

        let Some(paths) = rfd::FileDialog::new().set_directory(&folder).pick_files() else {
            return false;
        };

        if paths.is_empty() {
            return false;
        }

        let mut invalid_entries = Vec::new();
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

            match self.add_file_entry(kind, name) {
                Ok(_) => {
                    added_any = true;
                }
                Err(err) => {
                    invalid_entries.push(err);
                }
            }
        }

        if invalid_entries.is_empty() {
            if added_any {
                self.clear_error_message();
                self.status.clear();
            }
        } else {
            let message = format!("Some files could not be added to the {list_label} list");
            self.set_error_message((message, invalid_entries));
        }

        added_any
    }

    pub(crate) fn handle_add_file_from_entry(&mut self, kind: ListKind, entry: &str) -> bool {
        let list_label = kind.label();
        match self.add_file_entry(kind, entry) {
            Ok(_) => {
                self.clear_error_message();
                self.status.clear();
                true
            }
            Err(err) => {
                let message = format!("Could not add the entry to the {list_label} list");
                self.set_error_message((message, vec![err]));
                false
            }
        }
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

    fn add_file_entry(&mut self, kind: ListKind, entry: &str) -> Result<usize, String> {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            return Err("File name cannot be empty".to_string());
        }

        let (files, selected) = self.list_mut(kind);
        if files.iter().any(|existing| existing == trimmed) {
            return Err(format!("{trimmed} (already listed)"));
        }

        files.push(trimmed.to_string());
        let index = files.len() - 1;
        *selected = Some(index);
        Ok(index)
    }

    fn validate_icon_sys_settings(&self) -> Result<(), String> {
        if self.icon_sys_enabled && !self.icon_sys_use_existing {
            let line1 = &self.icon_sys_title_line1;
            let line2 = &self.icon_sys_title_line2;

            if line1.chars().count() > ICON_SYS_TITLE_CHAR_LIMIT {
                return Err(format!(
                    "Icon.sys line 1 cannot exceed {ICON_SYS_TITLE_CHAR_LIMIT} characters"
                ));
            }
            if line2.chars().count() > ICON_SYS_TITLE_CHAR_LIMIT {
                return Err(format!(
                    "Icon.sys line 2 cannot exceed {ICON_SYS_TITLE_CHAR_LIMIT} characters"
                ));
            }
            let title_is_valid = |value: &str| {
                !value.chars().any(|c| c.is_control()) && sjis::is_roundtrip_sjis(value)
            };
            if !title_is_valid(line1) || !title_is_valid(line2) {
                return Err(
                    "Icon.sys titles must contain characters representable in Shift-JIS"
                        .to_string(),
                );
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

        let mut exclude = self.exclude_files.clone();
        if !exclude.iter().any(|entry| entry == "psu.toml") {
            exclude.push("psu.toml".to_string());
        }
        let exclude = Some(exclude);

        let icon_sys = if self.icon_sys_enabled && !self.icon_sys_use_existing {
            let encoded_line1 = sjis::encode_sjis(&self.icon_sys_title_line1).map_err(|_| {
                "Icon.sys titles must contain characters representable in Shift-JIS".to_string()
            })?;
            let linebreak_pos = encoded_line1.len() as u16;
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

        if self.folder_base_name.trim().is_empty() {
            return Err("PSU name cannot be empty".to_string());
        }

        let name = self.folder_name();

        Ok(psu_packer::Config {
            name,
            timestamp: self.timestamp,
            include,
            exclude,
            icon_sys,
        })
    }

    #[cfg(feature = "psu-toml-editor")]
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

    #[cfg(not(feature = "psu-toml-editor"))]
    pub(crate) fn refresh_psu_toml_editor(&mut self) {
        self.psu_toml_sync_blocked = false;
    }
}
