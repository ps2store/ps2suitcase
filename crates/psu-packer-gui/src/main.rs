use eframe::egui;
use std::path::PathBuf;

struct PackerApp {
    folder: Option<PathBuf>,
    output: String,
    status: String,
}

impl Default for PackerApp {
    fn default() -> Self {
        Self {
            folder: None,
            output: String::new(),
            status: String::new(),
        }
    }
}

impl eframe::App for PackerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Select folder").clicked() {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    if let Ok(config) = psu_packer::load_config(&dir) {
                        self.output = format!("{}.psu", config.name);
                    }
                    self.folder = Some(dir);
                }
            }
            if let Some(folder) = &self.folder {
                ui.label(format!("Folder: {}", folder.display()));
            }
            ui.horizontal(|ui| {
                ui.label("Output:");
                ui.text_edit_singleline(&mut self.output);
                if ui.button("Browse").clicked() {
                    if let Some(file) = rfd::FileDialog::new()
                        .set_file_name(&self.output)
                        .save_file()
                    {
                        self.output = file.display().to_string();
                    }
                }
            });
            if ui.button("Pack").clicked() {
                if let Some(folder) = &self.folder {
                    let output_path = PathBuf::from(&self.output);
                    match psu_packer::pack_psu(folder, &output_path) {
                        Ok(_) => self.status = format!("Packed to {}", output_path.display()),
                        Err(e) => self.status = format!("Error: {e}"),
                    }
                } else {
                    self.status = "Please select a folder".to_string();
                }
            }
            if !self.status.is_empty() {
                ui.label(&self.status);
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "PSU Packer",
        options,
        Box::new(|_cc| Box::<PackerApp>::default()),
    )
}
