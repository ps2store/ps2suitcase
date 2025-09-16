use crate::tabs::Tab;
use crate::tabs::{ICNViewer, IconSysViewer, PsuTomlViewer, TitleCfgViewer};
use crate::AppState;
use eframe::egui::{Id, Ui, WidgetText};

pub struct TabViewer<'a> {
    pub(crate) app: &'a mut AppState,
}

pub enum TabType {
    IconSysViewer(IconSysViewer),
    TitleCfgViewer(TitleCfgViewer),
    ICNViewer(ICNViewer),
    PsuTomlViewer(PsuTomlViewer),
}

impl TabType {
    pub fn get_id(&self) -> &str {
        match self {
            TabType::IconSysViewer(tab) => tab.get_id(),
            TabType::TitleCfgViewer(tab) => tab.get_id(),
            TabType::ICNViewer(tab) => tab.get_id(),
            TabType::PsuTomlViewer(tab) => tab.get_id(),
        }
    }

    pub fn get_title(&self) -> String {
        match self {
            TabType::IconSysViewer(tab) => tab.get_title(),
            TabType::TitleCfgViewer(tab) => tab.get_title(),
            TabType::ICNViewer(tab) => tab.get_title(),
            TabType::PsuTomlViewer(tab) => tab.get_title(),
        }
    }

    pub fn get_modified(&self) -> bool {
        match self {
            TabType::IconSysViewer(tab) => tab.get_modified(),
            TabType::TitleCfgViewer(tab) => tab.get_modified(),
            TabType::ICNViewer(tab) => tab.get_modified(),
            TabType::PsuTomlViewer(tab) => tab.get_modified(),
        }
    }

    pub fn save(&mut self) {
        match self {
            TabType::IconSysViewer(tab) => tab.save(),
            TabType::TitleCfgViewer(tab) => tab.save(),
            TabType::ICNViewer(tab) => tab.save(),
            TabType::PsuTomlViewer(tab) => tab.save(),
        }
    }
}

impl<'a> egui_dock::TabViewer for TabViewer<'a> {
    type Tab = Box<TabType>;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        if tab.get_modified() {
            format!("* {}", tab.get_title())
        } else {
            tab.get_title()
        }
        .into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        match &mut **tab {
            TabType::IconSysViewer(tab) => {
                tab.show(ui, &mut self.app);
            }
            TabType::ICNViewer(tab) => {
                tab.show(ui);
            }
            TabType::TitleCfgViewer(tab) => {
                tab.show(ui);
            }
            TabType::PsuTomlViewer(tab) => {
                tab.show(ui);
            }
        }
    }

    fn id(&mut self, tab: &mut Self::Tab) -> Id {
        Id::new(tab.get_id())
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> bool {
        if let TabType::ICNViewer(tab) = &mut **tab {
            tab.closing = true;
        }
        true
    }

    fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
        false
    }
}
