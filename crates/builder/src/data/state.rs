use crate::data::virtual_file::VirtualFile;
use std::path::PathBuf;

#[derive(Clone)]
pub enum AppEvent {
    OpenFolder,
    OpenFile(VirtualFile),
    SetTitle(String),
    AddFiles,
    ExportPSU,
    SaveFile,
}

pub struct AppState {
    pub opened_folder: Option<PathBuf>,
    pub files: Vec<VirtualFile>,
    pub events: Vec<AppEvent>,
    pub calculated_size: u64,
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
            files: vec![],
            events: vec![],
            calculated_size: 0,
        }
    }
}
