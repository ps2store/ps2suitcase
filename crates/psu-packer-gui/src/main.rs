#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use psu_packer_gui::PackerApp;

trait NativeOptionsExt {
    fn with_centered(self, centered: bool) -> Self;
}

impl NativeOptionsExt for eframe::NativeOptions {
    fn with_centered(mut self, centered: bool) -> Self {
        self.centered = centered;
        self
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 768.0]),
        ..Default::default()
    }
    .with_centered(true);

    eframe::run_native(
        "PSU Packer",
        options,
        Box::new(|_cc| Box::<PackerApp>::default()),
    )
}
