use crate::data::files::Files;
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
    OpenSave,
    CreateICN,
    CreateTitleCfg,
    OpenSettings,
    StartPCSX2,
    StartPCSX2Elf(PathBuf),
    Validate,
}

pub struct AppState {
    pub opened_folder: Option<PathBuf>,
    pub files: Files,
    pub events: Vec<AppEvent>,
    pub pcsx2_path: String,
}

impl AppState {}

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
    pub fn create_icn(&mut self) {
        self.events.push(AppEvent::CreateICN);
    }
    pub fn create_title_cfg(&mut self) {
        self.events.push(AppEvent::CreateTitleCfg);
    }
    pub fn open_settings(&mut self) {
        self.events.push(AppEvent::OpenSettings);
    }
    pub fn start_pcsx2(&mut self) {
        self.events.push(AppEvent::StartPCSX2);
    }
    pub fn start_pcsx2_elf(&mut self, path: PathBuf) {
        self.events.push(AppEvent::StartPCSX2Elf(path));
    }
    pub fn validate(&mut self) {
        self.events.push(AppEvent::Validate);
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
            pcsx2_path: String::new(),
        }
    }
}
