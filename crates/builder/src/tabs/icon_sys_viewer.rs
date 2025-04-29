use crate::tabs::Tab;
use crate::{AppState, VirtualFile};
use eframe::egui::{ComboBox, Ui};
use ps2_filetypes::IconSys;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct IconSysViewer {
    app: Arc<Mutex<AppState>>,
    title: String,
    file: String,
    pub icon_file: String,
    pub icon_copy_file: String,
    pub icon_delete_file: String,
}

impl IconSysViewer {
    pub fn new(app: Arc<Mutex<AppState>>, file: Arc<Mutex<VirtualFile>>) -> Self {
        let file = file.clone();
        let mut file = file.lock().unwrap();
        let bytes = if let Some(file) = &mut file.file {
            let mut buf = Vec::new();
            file.seek(SeekFrom::Start(0)).unwrap();
            file.read_to_end(&mut buf).unwrap();
            buf
        } else {
            vec![]
        };
        let sys = IconSys::new(bytes);

        Self {
            app,
            title: sys.title.clone(),
            icon_file: sys.icon_file.clone(),
            icon_copy_file: sys.icon_copy_file.clone(),
            icon_delete_file: sys.icon_delete_file.clone(),
            file: file.name.clone(),
        }
    }
}

impl Tab for IconSysViewer {
    fn get_title(&self) -> String {
        self.file.clone()
    }

    fn get_content(&mut self, ui: &mut Ui) {
        let files: Vec<String> = self.app.lock().unwrap().files.clone().iter().filter_map(|file| {
            let name = file.lock().unwrap().name.clone();
            if matches!(PathBuf::from(&name).extension().unwrap().to_str().unwrap_or(""), "icn" | "ico") {
                Some(name.clone())
            } else {
                None
            }
        }).collect();

        ui.horizontal(|ui| {
            ui.label("Title");
            ui.text_edit_singleline(&mut self.title);
        });
        ui.horizontal(|ui| {
            ui.label("Icon");
            ComboBox::from_id_salt("icon").selected_text(&self.icon_file).show_ui(ui, |ui| {
                for file in files.iter() {
                    ui.selectable_value(&mut self.icon_file, file.clone(), file.clone());
                }
            });
        });

        ui.horizontal(|ui| {
            ui.label("Copy Icon");
            ComboBox::from_id_salt("copy_icon").selected_text(&self.icon_copy_file).show_ui(ui, |ui| {
                for file in files.iter() {
                    ui.selectable_value(&mut self.icon_file, file.clone(), file.clone());
                }
            });
        });

        ui.horizontal(|ui| {
            ui.label("Delete Icon");
            ComboBox::from_id_salt("delete_icon").selected_text(&self.icon_delete_file).show_ui(ui, |ui| {
                for file in files.iter() {
                    ui.selectable_value(&mut self.icon_file, file.clone(), file.clone());
                }
            });;
        });
    }
}
