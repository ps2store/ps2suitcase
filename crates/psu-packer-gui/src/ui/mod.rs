use eframe::egui;

pub mod dialogs;
pub mod file_picker;
pub mod icon_sys;
pub mod pack_controls;
pub mod theme;
pub mod timestamps;

pub(crate) fn centered_column<R>(
    ui: &mut egui::Ui,
    max_width: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let width = ui.available_width().min(max_width);
    ui.vertical_centered(|ui| {
        ui.set_width(width);
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            add_contents(ui)
        })
        .inner
    })
    .inner
}
