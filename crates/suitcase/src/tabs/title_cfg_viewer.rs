use crate::tabs::Tab;
use crate::VirtualFile;
use eframe::egui::Ui;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use relative_path::PathExt;
use crate::data::state::AppState;

pub struct TitleCfgViewer {
    file: String,
    file_path: PathBuf,
    contents: String,
    modified: bool,
    encoding_error: bool,
}

impl TitleCfgViewer {
    pub fn new(file: &VirtualFile, state: &AppState) -> Self {
        let buf = std::fs::read(&file.file_path)
            .expect("Failed to read file");

        let contents = String::from_utf8(buf).ok();
        let encoding_error = contents.is_none();

        Self {
            file: file
                .file_path
                .relative_to(state.opened_folder.clone().unwrap())
                .unwrap()
                .to_string(),
            file_path: file.file_path.clone(),
            contents: contents.unwrap_or_default(),
            encoding_error,
            modified: false,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("File:");
            ui.monospace(&self.file);
        });

        if self.encoding_error {
            ui.colored_label(eframe::egui::Color32::RED, "Encoding error, please use valid ASCII or UTF-8 encoding.");
        } else {
            if ui.text_edit_multiline(&mut self.contents).changed() {
                self.modified = true;
            }
        }

        if ui.button("Save").clicked() {
            self.save();
        }
    }
}

impl Tab for TitleCfgViewer {
    fn get_id(&self) -> &str {
        &self.file
    }
    fn get_title(&self) -> String {
        self.file.to_string()
    }


    fn get_modified(&self) -> bool {
        self.modified
    }

    fn save(&mut self) {
        let mut output = File::create(&self.file_path).expect("File not found");
        output.write_all(self.contents.as_bytes()).unwrap();
        self.modified = false;
    }
}
