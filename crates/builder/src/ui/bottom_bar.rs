pub struct BottomBar;

impl eframe::egui::Widget for BottomBar {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
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
}
