use eframe::egui::{include_image, menu, Response, Ui, Vec2, Widget};
use egui_dock::egui::Button;

pub struct Toolbar {

}

impl Widget for Toolbar {
    fn ui(self, ui: &mut Ui) -> Response {
        menu::bar(ui, |ui| {
            ui.set_min_size(Vec2::new(24.0, 24.0));
            ui.add(Button::image(include_image!("../../assets/lowdpi/main_open_dir.png")));
            ui.add(Button::image(include_image!("../../assets/lowdpi/main_open_sav.png")));
            ui.add(Button::image(include_image!("../../assets/lowdpi/main_open_vmc.png")));
            ui.add(Button::image(include_image!("../../assets/lowdpi/main_mk_titlecfg.png")));
            ui.add(Button::image(include_image!("../../assets/lowdpi/main_mk_iconsys.png")));
            ui.add(Button::image(include_image!("../../assets/lowdpi/main_sav_meta.png")));
            ui.add(Button::image(include_image!("../../assets/lowdpi/main_extract_all.png")));
            ui.add(Button::image(include_image!("../../assets/lowdpi/main_mk_sav.png")));
            ui.add(Button::image(include_image!("../../assets/lowdpi/main_mk_vmc.png")));
            ui.add(Button::image(include_image!("../../assets/lowdpi/main_valid_ok.png")));
        }).response
    }
}