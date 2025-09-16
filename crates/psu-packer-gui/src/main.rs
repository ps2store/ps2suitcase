#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use psu_packer_gui::PackerApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "PSU Packer",
        options,
        Box::new(|_cc| Box::<PackerApp>::default()),
    )
}
