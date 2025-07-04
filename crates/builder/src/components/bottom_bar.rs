use crate::data::state::AppState;
use bytesize::ByteSize;
use eframe::egui::{Color32, Ui};

fn size_label(ui: &mut Ui, size: u64) {
    ui.label("Size: ");
    ui.colored_label(
        if size > 8 * 1024 * 1024 {
            Color32::RED
        } else {
            Color32::WHITE
        },
        ByteSize::b(size).to_string(),
    );
}

pub fn bottom_bar(ui: &mut Ui, app: &mut AppState) -> eframe::egui::Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        size_label(ui, app.calculated_size);
        ui.add_space(5.0);
        ui.separator();
        ui.label("Version: ");
        ui.label(env!("CARGO_PKG_VERSION"));
        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);
        ui.label("Made by ");
        ui.hyperlink_to("tech", "https://github.com/simonhochrein");
        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);
        ui.hyperlink_to("support me", "https://ko-fi.com/techwritescode/");
    })
    .response
}
