use eframe::egui::Ui;
use crate::components::menu_bar::OPEN_FOLDER_KEYBOARD_SHORTCUT;
use crate::data::state::AppState;

pub fn greeting(ui: &mut Ui, app: &mut AppState) {
    let is_folder_open = app.opened_folder.is_some();

    ui.centered_and_justified(|ui| {
        if !is_folder_open {
            ui.heading(format!(
                "Open a folder to get started ({})",
                &ui.ctx().format_shortcut(&OPEN_FOLDER_KEYBOARD_SHORTCUT)
            ));
        } else {
            ui.heading("No open editors");
        }
    });
}