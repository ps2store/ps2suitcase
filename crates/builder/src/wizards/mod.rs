use crate::wizards::create_icn::CreateICN;
use crate::wizards::wizard::Wizard;
use eframe::egui::Context;

pub mod create_icn;
pub mod wizard;

pub trait Wizards {
    fn create_icn_wizard(&self, show: &mut bool);
}

impl Wizards for &Context {
    fn create_icn_wizard(&self, show: &mut bool) {
        CreateICN {}.show_modal(self, show);
    }
}
