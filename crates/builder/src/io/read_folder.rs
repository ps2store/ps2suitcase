use crate::data::virtual_file::VirtualFile;
use std::path::PathBuf;

pub fn read_folder(folder: PathBuf) -> std::io::Result<Vec<VirtualFile>> {
    let mut files = std::fs::read_dir(folder)?
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

    files.sort_by(|a, b| a.name.partial_cmp(&b.name).unwrap());

    Ok(files)
}
