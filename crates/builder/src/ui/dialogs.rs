use std::path::PathBuf;
use eframe::egui::{Context, Response};

pub trait Dialogs {
    fn save_as(&self, filename: impl Into<String>) -> Option<PathBuf>;
    fn open_file(&self) -> Option<PathBuf>;
    fn open_files(&self) -> Option<Vec<PathBuf>>;
}

impl Dialogs for &Context {
    fn save_as(&self, filename: impl Into<String>) -> Option<PathBuf> {
        rfd::FileDialog::new().set_file_name(filename).save_file()
    }
    
    fn open_file(&self) -> Option<PathBuf> {
        rfd::FileDialog::new().pick_file()
    }
    
    fn open_files(&self) -> Option<Vec<PathBuf>> {
        rfd::FileDialog::new().pick_files()
    }
}