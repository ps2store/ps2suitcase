use crate::data::files::Files;
use crate::data::virtual_file::VirtualFile;
use std::path::PathBuf;

pub fn read_folder(folder: PathBuf) -> std::io::Result<Files> {
    let files = std::fs::read_dir(folder)?
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            if entry.file_type().ok()?.is_file() {
                Some(VirtualFile {
                    name: entry.file_name().into_string().unwrap(),
                    file_path: entry.path(),
                    size: entry.path().metadata().unwrap().len(),
                })
            } else {
                None
            }
        })
        .collect::<Vec<VirtualFile>>();

    Ok(Files::from(files)?)
}
