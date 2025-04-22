mod psu;

use crate::psu::PSU;
use eframe::egui;
use eframe::egui::{Align, Color32};
use std::fs::{read_dir, File};
use std::path::PathBuf;

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 400.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "PS2 PSU Exporter",
        options,
        Box::new(|_cc| Ok(Box::<MCM>::default())),
    )
    .unwrap()
}

#[derive(Default)]
struct MCM {
    dir: PathBuf,
    files: Vec<PathBuf>,
}

impl eframe::App for MCM {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("Load folder").clicked() {
                        if let Some(folder) = rfd::FileDialog::new()
                            .set_title("Pick Folder")
                            .pick_folder()
                        {
                            self.dir = folder.clone();
                            let dir = read_dir(folder.as_path()).expect("Could not read folder");
                            self.files = vec![];
                            for file in dir.flatten() {
                                let path = file.path();
                                self.files.push(path);
                            }
                        }
                    }
                    if !self.files.is_empty() {
                        if ui.button("Export PSU").clicked() {
                            let mut psu = PSU::new();
                            for file in &self.files {
                                if file.is_file() {
                                    psu.add_file(
                                        file.file_name().unwrap().to_str().unwrap().to_string(),
                                        &mut File::open(file.as_path())
                                            .expect("Could not open file"),
                                    )
                                    .expect("Could not read file");
                                }
                            }
                            let suggested =
                                self.dir.file_name().unwrap().to_str().unwrap().to_string()
                                    + ".PSU";
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name(suggested)
                                .set_title("Export PSU")
                                .save_file()
                            {
                                psu.write(&path).unwrap();
                            }
                        }
                    }
                });
                ui.add_space(10.0);
                if !self.files.is_empty() {
                    ui.label(format!("Loaded: {}", self.dir.to_str().unwrap()));
                }
                ui.add_space(10.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.with_layout(
                            egui::Layout::top_down_justified(Align::LEFT).with_cross_justify(true),
                            |ui| {
                                for file in &self.files {
                                    let legal = file.is_file();

                                    ui.colored_label(
                                        if legal { Color32::GREEN } else { Color32::RED },
                                        file.file_name().unwrap().to_str().unwrap(),
                                    );
                                }
                            },
                        );
                    });
            });
        });
    }
}
