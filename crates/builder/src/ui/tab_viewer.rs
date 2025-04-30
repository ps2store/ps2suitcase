use crate::tabs::Tab;
use eframe::egui::{Id, Ui, WidgetText};

pub struct TabViewer {}

impl egui_dock::TabViewer for TabViewer {
    type Tab = Box<dyn Tab>;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        if tab.get_modified() {
            format!("* {}", tab.get_title())
        } else {
            tab.get_title()
        }.into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        tab.get_content(ui);
    }

    fn id(&mut self, tab: &mut Self::Tab) -> Id {
        Id::new(tab.get_id())
    }
}
