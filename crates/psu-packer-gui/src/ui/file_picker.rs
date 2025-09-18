use std::{
    fs,
    path::{Path, PathBuf},
};

use eframe::egui;
use ps2_filetypes::{IconSys, PSUEntryKind, PSU};

use crate::{ui::theme, PackerApp, SasPrefix, TimestampStrategy};

pub(crate) fn file_menu(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.menu_button("File", |ui| {
        file_menu_contents(app, ui, None);
    });
}

fn file_menu_contents(
    app: &mut PackerApp,
    ui: &mut egui::Ui,
    mut recorder: Option<&mut dyn FileMenuRecorder>,
) {
    let pack_in_progress = app.is_pack_running();

    let pack_psu_response = ui
        .add_enabled(!pack_in_progress, egui::Button::new("Pack PSU"))
        .on_hover_text("Create the PSU archive using the settings above.");
    if let Some(recorder) = recorder.as_mut() {
        recorder.record(FileMenuItem::PackPsu, pack_psu_response.enabled());
    }
    if pack_psu_response.clicked() {
        app.handle_pack_request();
        ui.close_menu();
    }

    if ui.button("Save PSU As...").clicked() {
        app.browse_output_destination();
        ui.close_menu();
    }

    if ui.button("Open PSU...").clicked() {
        app.handle_open_psu();
        ui.close_menu();
    }

    let edit_psu_response = ui.button("Edit psu.toml");
    if let Some(recorder) = recorder.as_mut() {
        recorder.record(FileMenuItem::EditPsuToml, edit_psu_response.enabled());
    }
    if edit_psu_response.clicked() {
        app.open_psu_toml_tab();
        ui.close_menu();
    }

    let edit_title_response = ui.button("Edit title.cfg");
    if let Some(recorder) = recorder.as_mut() {
        recorder.record(FileMenuItem::EditTitleCfg, edit_title_response.enabled());
    }
    if edit_title_response.clicked() {
        app.open_title_cfg_tab();
        ui.close_menu();
    }

    let edit_icon_response = ui.button("Edit icon.sys");
    if let Some(recorder) = recorder.as_mut() {
        recorder.record(FileMenuItem::EditIconSys, edit_icon_response.enabled());
    }
    if edit_icon_response.clicked() {
        app.open_icon_sys_tab();
        ui.close_menu();
    }

    let create_psu_response = ui.button("Create psu.toml from template");
    if let Some(recorder) = recorder.as_mut() {
        recorder.record(FileMenuItem::CreatePsuToml, create_psu_response.enabled());
    }
    if create_psu_response.clicked() {
        app.create_psu_toml_from_template();
        ui.close_menu();
    }

    let create_title_response = ui.button("Create title.cfg from template");
    if let Some(recorder) = recorder.as_mut() {
        recorder.record(
            FileMenuItem::CreateTitleCfg,
            create_title_response.enabled(),
        );
    }
    if create_title_response.clicked() {
        app.create_title_cfg_from_template();
        ui.close_menu();
    }

    ui.separator();

    if ui.button("Exit").clicked() {
        app.show_exit_confirm = true;
        app.exit_confirmed = false;
        ui.close_menu();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum FileMenuItem {
    PackPsu,
    EditPsuToml,
    EditTitleCfg,
    EditIconSys,
    CreatePsuToml,
    CreateTitleCfg,
}

trait FileMenuRecorder {
    fn record(&mut self, item: FileMenuItem, enabled: bool);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PackerApp;
    use eframe::egui;
    use std::collections::HashMap;

    #[test]
    fn file_menu_buttons_enabled_without_folder() {
        let mut app = PackerApp::default();
        assert!(app.folder.is_none());

        let ctx = egui::Context::default();
        let mut recorder = RecordingMenuRecorder::default();

        ctx.begin_frame(egui::RawInput::default());
        egui::CentralPanel::default().show(&ctx, |ui| {
            file_menu_contents(&mut app, ui, Some(&mut recorder));
        });
        let _ = ctx.end_frame();

        assert!(recorder.is_enabled(FileMenuItem::PackPsu));
        assert!(recorder.is_enabled(FileMenuItem::EditPsuToml));
        assert!(recorder.is_enabled(FileMenuItem::EditTitleCfg));
        assert!(recorder.is_enabled(FileMenuItem::EditIconSys));
        assert!(recorder.is_enabled(FileMenuItem::CreatePsuToml));
        assert!(recorder.is_enabled(FileMenuItem::CreateTitleCfg));
    }

    #[derive(Default)]
    struct RecordingMenuRecorder {
        entries: HashMap<FileMenuItem, bool>,
    }

    impl RecordingMenuRecorder {
        fn is_enabled(&self, item: FileMenuItem) -> bool {
            *self.entries.get(&item).unwrap_or(&false)
        }
    }

    impl FileMenuRecorder for RecordingMenuRecorder {
        fn record(&mut self, item: FileMenuItem, enabled: bool) {
            self.entries.insert(item, enabled);
        }
    }
}

pub(crate) fn folder_section(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.group(|ui| {
        ui.heading(theme::display_heading_text(ui, "Folder"));
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
                    load_project_files(app, &folder);
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
            if !app.missing_required_project_files.is_empty() {
                let warning = PackerApp::format_missing_required_files_message(
                    &app.missing_required_project_files,
                );
                ui.colored_label(egui::Color32::YELLOW, warning);
            }
        }
    });
}

pub(crate) fn loaded_psu_section(app: &PackerApp, ui: &mut egui::Ui) {
    ui.group(|ui| {
        ui.heading(theme::display_heading_text(ui, "Loaded PSU"));
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

pub(crate) fn load_project_files(app: &mut PackerApp, folder: &Path) {
    app.load_timestamp_rules_from_folder(folder);
    match psu_packer::load_config(folder) {
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
            if let Some(default_path) = app.default_output_path_with(Some(folder)) {
                app.output = default_path.display().to_string();
            } else {
                app.output.clear();
            }
            app.source_timestamp = timestamp;
            app.include_files = include.unwrap_or_default();
            app.exclude_files = exclude.unwrap_or_default();
            app.selected_include = None;
            app.selected_exclude = None;
            app.clear_error_message();
            app.status.clear();

            let mut parsed_icon_sys = None;
            if let Some(icon_sys_path) = find_icon_sys_path(folder) {
                match fs::read(&icon_sys_path) {
                    Ok(bytes) => match std::panic::catch_unwind(|| IconSys::new(bytes)) {
                        Ok(icon_sys) => parsed_icon_sys = Some(icon_sys),
                        Err(_) => {
                            app.set_error_message(format!(
                                "Failed to parse {} as an icon.sys file.",
                                icon_sys_path.display()
                            ));
                        }
                    },
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
            let message = format_load_error(folder, err);
            app.set_error_message(message);
            app.output.clear();
            app.selected_prefix = SasPrefix::default();
            app.folder_base_name.clear();
            app.psu_file_base_name.clear();
            app.timestamp = None;
            app.timestamp_strategy = TimestampStrategy::None;
            app.timestamp_from_rules = false;
            app.source_timestamp = None;
            app.manual_timestamp = None;
            app.include_files.clear();
            app.exclude_files.clear();
            app.selected_include = None;
            app.selected_exclude = None;
            app.reset_icon_sys_fields();
        }
    }
    app.loaded_psu_path = None;
    app.loaded_psu_files.clear();
    app.folder = Some(folder.to_path_buf());
    app.sync_timestamp_after_source_update();
    app.reload_project_files();
}

fn find_icon_sys_path(folder: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(folder).ok()?;
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.eq_ignore_ascii_case("icon.sys"))
                    .unwrap_or(false)
        })
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
        self.source_timestamp = root_timestamp;
        self.loaded_psu_files = files;
        self.loaded_psu_path = Some(path.clone());
        self.clear_error_message();
        self.status = format!("Loaded PSU from {}", path.display());
        self.folder = None;
        self.missing_required_project_files.clear();
        self.sync_timestamp_after_source_update();
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
