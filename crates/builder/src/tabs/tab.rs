use eframe::egui::Ui;

pub trait Tab {
    fn get_id(&self) -> &str;
    fn get_title(&self) -> String;
    fn get_content(&mut self, ui: &mut Ui);
    fn get_modified(&self) -> bool;

    fn save(&mut self);
}
