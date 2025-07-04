use std::path::PathBuf;

#[derive(Clone)]
pub struct VirtualFile {
    pub name: String,
    pub file_path: PathBuf,
    pub size: u64,
}
