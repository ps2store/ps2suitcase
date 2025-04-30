use crate::tabs::Tab;
use crate::{AppState, VirtualFile};
use eframe::egui::{ComboBox, Ui};
use ps2_filetypes::IconSys;
use std::fs::File;
use std::io::Read;
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
        let virtual_file = file.clone();
        let virtual_file = virtual_file.lock().unwrap();
        let mut file = File::open(&virtual_file.file_path).expect("File not found");
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();

        let sys = IconSys::new(buf);

        Self {
            app,
            title: sys.title.clone(),
            icon_file: sys.icon_file.clone(),
            icon_copy_file: sys.icon_copy_file.clone(),
            icon_delete_file: sys.icon_delete_file.clone(),
            file: virtual_file.file_path.file_name().unwrap().to_str().unwrap().to_string(),
        }
    }
}

impl Tab for IconSysViewer {
    fn get_id(&self) -> &str {
        &self.file
    }

    fn get_title(&self) -> String {
        self.file.clone()
    }

    fn get_content(&mut self, ui: &mut Ui) {
        let files: Vec<String> = self
            .app
            .lock()
            .unwrap()
            .files
            .clone()
            .iter()
            .filter_map(|file| {
                let name = file.lock().unwrap().name.clone();
                if matches!(
                    PathBuf::from(&name)
                        .extension()
                        .unwrap()
                        .to_str()
                        .unwrap_or(""),
                    "icn" | "ico"
                ) {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        ui.horizontal(|ui| {
            ui.label("Title");
            ui.text_edit_singleline(&mut self.title);
        });
        ui.horizontal(|ui| {
            ui.label("Icon");
            file_select(ui, "list_icon", &mut self.icon_file, &files);
        });
        ui.horizontal(|ui| {
            ui.label("Copy Icon");
            file_select(ui, "copy_icon", &mut self.icon_copy_file, &files);
        });
        ui.horizontal(|ui| {
            ui.label("Delete Icon");
            file_select(ui, "delete_icon", &mut self.icon_delete_file, &files);
        });
    }

    fn get_modified(&self) -> bool {
        false
    }

    fn save(&mut self) {
        todo!()
    }
}

fn file_select(ui: &mut Ui, name: impl Into<String>, value: &mut String, files: &[String]) {
    ComboBox::from_id_salt(name.into())
        .selected_text(&*value)
        .show_ui(ui, |ui| {
            files.iter().for_each(|file| {
                ui.selectable_value(value, file.clone(), file.clone());
            });
        });
}
