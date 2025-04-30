use eframe::egui::{Button, KeyboardShortcut, OpenUrl, Response, Ui, WidgetText};
use crate::utils::shortcut;

pub trait MenuItemComponent {
    fn menu_item(self, label: impl Into<WidgetText>) -> Response;
    fn menu_item_link(self, label: impl Into<WidgetText>, link: &str) -> Response;
    fn menu_item_shortcut(self, label: impl Into<WidgetText>, shortcut: &KeyboardShortcut) -> Response;
}

impl MenuItemComponent for &mut Ui {
    fn menu_item(self, label: impl Into<WidgetText>) -> Response {
        self.button(label)
    }

    fn menu_item_link(self, label: impl Into<WidgetText>, link: &str) -> Response {
        let response = self.button(label);
        if response.clicked() {
            self.ctx().open_url(OpenUrl::new_tab(link));
            self.close_menu();
        }
        
        response
    }

    fn menu_item_shortcut(self, label: impl Into<WidgetText>, shortcut: &KeyboardShortcut) -> Response {
        let response = self.add(Button::new(label).shortcut_text(self.ctx().format_shortcut(shortcut)));
        if response.clicked() {
            self.close_menu();
        }
        
        response
    }
}