use crate::tabs::tab::Tab;
use crate::AppState;
use eframe::egui;
use eframe::egui::{Align, Button, Color32, Layout, TextStyle, Ui, Vec2, WidgetText};
use std::sync::{Arc, Mutex};
use eframe::egui::WidgetText::LayoutJob;

pub struct FileTree {
    pub state: Arc<Mutex<AppState>>,
}
impl Tab for FileTree {
    fn get_title(&self) -> String {
        "Files".to_owned()
    }

    fn get_content(&mut self, ui: &mut Ui) {
        ui.add_space(10.0);
        let mut state = self.state.lock().unwrap();
        ui.with_layout(Layout::default().with_cross_justify(true).with_main_align(Align::Center), |ui| {
            ui.scope(|ui| {
                ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::DARK_GREEN;
                ui.visuals_mut().widgets.hovered.weak_bg_fill = Color32::from_rgb(0, 0x6C, 0);
                ui.visuals_mut().widgets.active.weak_bg_fill = Color32::from_rgb(0, 0x68, 0);
                ui.spacing_mut().button_padding = Vec2::splat(10.0);

                let button = ui.add(Button::new(WidgetText::from("➕Add Files").heading()));
                if button.clicked() {}
            });
        });
        ui.add_space(10.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.with_layout(Layout::top_down_justified(Align::Min), |ui| {
                for file in state.files.clone().into_iter() {
                    let name = file.lock().unwrap().name.clone();
                    if ui.selectable_label(false, name).clicked() {
                        state.open_file(file);
                    }
                }
            });
        });
    }
}
