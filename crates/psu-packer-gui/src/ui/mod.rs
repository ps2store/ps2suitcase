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
    let available = ui.available_width();
    let width = available.min(max_width);
    let margin = ((available - width) * 0.5).max(0.0);

    let mut result = None;
    ui.horizontal(|ui| {
        if margin > 0.0 {
            ui.add_space(margin);
        }

        result = Some(
            ui.scope(|ui| {
                ui.set_width(width);
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    add_contents(ui)
                })
                .inner
            })
            .inner,
        );

        if margin > 0.0 {
            ui.add_space(margin);
        }
    });

    result.expect("centered_column should always produce a result")
}
