use crate::wizards::wizard::Wizard;
use eframe::egui::{Response, Ui, Widget};
use std::hash::Hash;

pub struct CreateICN {}

impl Widget for &mut CreateICN {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.button("Create ICN")
    }
}

impl Wizard for &mut CreateICN {
    fn get_id(&self) -> impl Hash {
        "create_icn"
    }
}
