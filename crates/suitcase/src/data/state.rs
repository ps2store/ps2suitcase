use crate::data::virtual_file::VirtualFile;
use std::path::PathBuf;
use crate::data::files::Files;

#[derive(Clone)]
pub enum AppEvent {
    OpenFolder,
    OpenFile(VirtualFile),
    SetTitle(String),
    AddFiles,
    ExportPSU,
    SaveFile,
    OpenSave,
}

pub struct AppState {
    pub opened_folder: Option<PathBuf>,
    pub files: Files,
    pub events: Vec<AppEvent>,
}

impl AppState {
    pub fn open_file(&mut self, file: VirtualFile) {
        self.events.push(AppEvent::OpenFile(file));
    }
    pub fn set_title(&mut self, title: String) {
        self.events.push(AppEvent::SetTitle(title));
    }
    pub fn add_files(&mut self) {
        self.events.push(AppEvent::AddFiles);
    }
    pub fn open_folder(&mut self) {
        self.events.push(AppEvent::OpenFolder);
    }
    pub fn open_save(&mut self) {
        self.events.push(AppEvent::OpenSave);
    }
    pub fn export_psu(&mut self) {
        self.events.push(AppEvent::ExportPSU);
    }
    pub fn save_file(&mut self) {
        self.events.push(AppEvent::SaveFile);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            opened_folder: None,
            files: Files::default(),
            events: vec![],
        }
    }
}
