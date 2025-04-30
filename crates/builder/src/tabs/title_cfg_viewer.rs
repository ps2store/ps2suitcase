use crate::tabs::Tab;
use crate::{AppState, VirtualFile};
use eframe::egui::Ui;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct TitleCfgViewer {
    file: String,
    file_path: PathBuf,
    contents: String,
    modified: bool,
}

impl TitleCfgViewer {
    pub fn new(app: Arc<Mutex<AppState>>, file: Arc<Mutex<VirtualFile>>) -> Self {
        let virtual_file = file.lock().unwrap();
        let mut file = File::open(&virtual_file.file_path).expect("File not found");
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();

        let contents = String::from_utf8(buf).unwrap();

        Self {
            file: virtual_file.name.clone(),
            file_path: virtual_file.file_path.clone(),
            contents,
            modified: false,
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

    fn get_content(&mut self, ui: &mut Ui) {
        ui.centered_and_justified(|ui| {
            if ui.text_edit_multiline(&mut self.contents).changed() {
                self.modified = true;
            }
        });
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
