use crate::tabs::Tab;
use eframe::egui::{Ui, WidgetText};

pub struct TabViewer {}

impl egui_dock::TabViewer for TabViewer {
    type Tab = Box<dyn Tab>;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.get_title().into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        tab.get_content(ui);
    }
}
