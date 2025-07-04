use eframe::egui::{include_image, menu, ImageSource, Response, Ui, Vec2, Widget};
use egui_dock::egui::Button;

pub struct Toolbar {

}

fn toolbar_item(ui: &mut Ui, source: ImageSource, tooltip: impl Into<String>) -> Response {
    ui.add(Button::image(source)).on_hover_ui(|ui| {
        ui.label(tooltip.into());
    })
}

impl Widget for Toolbar {
    fn ui(self, ui: &mut Ui) -> Response {
        menu::bar(ui, |ui| {
            ui.set_min_size(Vec2::new(24.0, 24.0));
            toolbar_item(ui, include_image!("../../assets/lowdpi/main_open_dir.png"), "Open a directory");
            toolbar_item(ui, include_image!("../../assets/lowdpi/main_open_sav.png"), "Open a save file");
            toolbar_item(ui, include_image!("../../assets/lowdpi/main_open_vmc.png"), "Open a virtual memory card");
            toolbar_item(ui, include_image!("../../assets/lowdpi/main_mk_titlecfg.png"), "Make title configuration");
            toolbar_item(ui, include_image!("../../assets/lowdpi/main_mk_iconsys.png"), "Make icon system");
            toolbar_item(ui, include_image!("../../assets/lowdpi/main_sav_meta.png"), "Save metadata");
            toolbar_item(ui, include_image!("../../assets/lowdpi/main_extract_all.png"), "Extract all saves");
            toolbar_item(ui, include_image!("../../assets/lowdpi/main_mk_sav.png"), "Make save file");
            toolbar_item(ui, include_image!("../../assets/lowdpi/main_mk_vmc.png"), "Make virtual memory card");
            toolbar_item(ui, include_image!("../../assets/lowdpi/main_valid_ok.png"), "Validate save file");
        }).response
    }
}