use crate::components::dialogs::{Dialogs, Filters};
use eframe::egui::{CornerRadius, Response, TextEdit, Ui};
use std::ops::Add;

pub trait FilePicker {
    fn file_picker(&mut self, value: &mut String, filters: Filters) -> Response;
}

fn set_border_radius(ui: &mut Ui, radius: CornerRadius) {
    ui.style_mut().visuals.widgets.hovered.corner_radius = radius.add(CornerRadius::same(1));
    ui.style_mut().visuals.widgets.inactive.corner_radius = radius;
    ui.style_mut().visuals.widgets.active.corner_radius = radius;
}

impl FilePicker for Ui {
    fn file_picker(&mut self, value: &mut String, filters: Filters) -> Response {
        self.horizontal(|ui| {
            ui.style_mut().spacing.item_spacing.x = 1.0;

            let width = ui.available_width();

            set_border_radius(ui, CornerRadius{nw: 2, sw: 2, ne: 0, se: 0});
            let response = ui.add(TextEdit::singleline(value).desired_width(width - 26.0));

            set_border_radius(ui, CornerRadius{nw: 0, sw: 0, ne: 2, se: 2});
            if ui.button("🗁").clicked() {
                if let Some(file) = ui.ctx().open_file(filters) {
                    *value = file.to_str().unwrap_or_default().to_string();
                }
            }

            response
        }).inner
    }
}