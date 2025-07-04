use std::path::Path;
use std::process::Command;

#[cfg(target_os = "windows")]
pub fn reveal_file_in_explorer(path: &Path) {
    let _ = Command::new("explorer")
        .arg("/select,")
        .arg(path)
        .status();
}

#[cfg(target_os = "macos")]
pub fn reveal_file_in_explorer(path: &Path) {
    let _ = Command::new("open")
        .arg("-R")
        .arg(path)
        .status();
}

#[cfg(target_os = "linux")]
pub fn reveal_file_in_explorer(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = Command::new("xdg-open")
            .arg(parent)
            .status();
    }
}