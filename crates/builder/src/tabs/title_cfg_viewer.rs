use std::fs::File;
use std::io::Read;
use std::sync::{Arc, Mutex};
use eframe::egui::Ui;
use crate::{AppState, VirtualFile};
use crate::tabs::Tab;

pub struct TitleCfgViewer {
    file: String,
    contents: String,
}

impl TitleCfgViewer {
    pub fn new(app: Arc<Mutex<AppState>>, file: Arc<Mutex<VirtualFile>>) -> Self {
        let mut contents: Vec<u8> = Vec::new();
        if let Some(file) = &mut file.lock().unwrap().file {
            file.read_to_end(&mut contents).unwrap();
        }
        
        let contents = String::from_utf8(contents).unwrap();
        
        Self {
            file: file.lock().unwrap().name.clone(),
            contents,
        }
    }
}

impl Tab for TitleCfgViewer {
    fn get_title(&self) -> String {
        self.file.to_string()
    }

    fn get_content(&mut self, ui: &mut Ui) {
        ui.centered_and_justified(|ui| {
            ui.text_edit_multiline(&mut self.contents);
        });
    }
}