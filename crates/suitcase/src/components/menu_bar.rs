use crate::components::menu_item::MenuItemComponent;
use crate::data::state::AppState;
use eframe::egui;
use eframe::egui::{menu, Context, KeyboardShortcut, Modifiers, Ui};

const CTRL_OR_CMD: Modifiers = if cfg!(target_os = "macos") {
    Modifiers::MAC_CMD
} else {
    Modifiers::CTRL
};
const CTRL_OR_CMD_SHIFT: Modifiers = if cfg!(target_os = "macos") {
    Modifiers {
        alt: false,
        ctrl: false,
        shift: true,
        mac_cmd: true,
        command: false,
    }
} else {
    Modifiers {
        alt: false,
        ctrl: true,
        shift: true,
        mac_cmd: false,
        command: false,
    }
};

pub const OPEN_FOLDER_KEYBOARD_SHORTCUT: KeyboardShortcut =
    KeyboardShortcut::new(CTRL_OR_CMD, egui::Key::O);
const EXPORT_KEYBOARD_SHORTCUT: KeyboardShortcut =
    KeyboardShortcut::new(CTRL_OR_CMD_SHIFT, egui::Key::S);
const ADD_FILE_KEYBOARD_SHORTCUT: KeyboardShortcut =
    KeyboardShortcut::new(CTRL_OR_CMD, egui::Key::N);
const SAVE_KEYBOARD_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(CTRL_OR_CMD, egui::Key::S);
const CREATE_ICN_KEYBOARD_SHORTCUT: KeyboardShortcut =
    KeyboardShortcut::new(CTRL_OR_CMD, egui::Key::I);

const OPEN_SETTINGS_KEYBOARD_SHORTCUT: KeyboardShortcut =
    KeyboardShortcut::new(CTRL_OR_CMD, egui::Key::Comma);

pub fn menu_bar(ui: &mut Ui, app: &mut AppState) {
    let is_folder_open = app.opened_folder.is_some();

    menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui
                .menu_item_shortcut("Open Folder", &OPEN_FOLDER_KEYBOARD_SHORTCUT)
                .clicked()
            {
                app.open_folder();
                ui.close_menu();
            }
            ui.add_enabled_ui(is_folder_open, |ui| {
                if ui
                    .menu_item_shortcut("Add Files", &ADD_FILE_KEYBOARD_SHORTCUT)
                    .clicked()
                {
                    app.add_files();
                    ui.close_menu();
                }
                if ui
                    .menu_item_shortcut("Save File", &SAVE_KEYBOARD_SHORTCUT)
                    .clicked()
                {
                    app.save_file();
                    ui.close_menu();
                }
                // ui.separator();
                // if ui
                //     .menu_item_shortcut("Create ICN", &CREATE_ICN_KEYBOARD_SHORTCUT)
                //     .clicked()
                // {
                //     self.show_create_icn = true;
                // }
            });
        });
        ui.menu_button("Edit", |ui| {
            if ui
                .menu_item_shortcut("Settings", &OPEN_SETTINGS_KEYBOARD_SHORTCUT)
                .clicked()
            {
                app.open_settings();
                ui.close_menu();
            }
        });
        ui.menu_button("Export", |ui| {
            ui.add_enabled_ui(is_folder_open, |ui| {
                if ui
                    .menu_item_shortcut("Export PSU", &EXPORT_KEYBOARD_SHORTCUT)
                    .clicked()
                {
                    app.export_psu();
                    ui.close_menu();
                }
            });
        });
        ui.menu_button("Help", |ui| {
            ui.menu_item_link("GitHub", "https://github.com/techwritescode/ps2-rust")
        })
    });
}

pub fn handle_accelerators(ctx: &Context, app: &mut AppState) {
    if ctx.input_mut(|i| i.consume_shortcut(&OPEN_FOLDER_KEYBOARD_SHORTCUT)) {
        app.open_folder();
    } else if ctx.input_mut(|i| i.consume_shortcut(&EXPORT_KEYBOARD_SHORTCUT)) {
        app.export_psu();
    } else if ctx.input_mut(|i| i.consume_shortcut(&SAVE_KEYBOARD_SHORTCUT)) {
        app.save_file();
    } else if ctx.input_mut(|i| i.consume_shortcut(&CREATE_ICN_KEYBOARD_SHORTCUT)) {
    } else if ctx.input_mut(|i| i.consume_shortcut(&ADD_FILE_KEYBOARD_SHORTCUT)) {
        app.add_files();
    } else if ctx.input_mut(|i| i.consume_shortcut(&OPEN_SETTINGS_KEYBOARD_SHORTCUT)) {
        app.open_settings();
    }
}
