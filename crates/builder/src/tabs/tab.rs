use eframe::egui::Ui;

pub trait Tab {
    fn get_title(&self) -> String;
    fn get_content(&mut self, ui: &mut Ui);
}
