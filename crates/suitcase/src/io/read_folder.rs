use crate::data::files::Files;
use crate::data::virtual_file::VirtualFile;
use std::path::PathBuf;

pub fn read_folder(folder: PathBuf) -> std::io::Result<Files> {
    let files = std::fs::read_dir(&folder)?
        .into_iter()
        .filter_map(|entry| match entry {
            Ok(entry) => {
                if entry.file_type().ok()?.is_file() {
                    let file_path = entry.path();
                    let name = match entry.file_name().into_string() {
                        Ok(name) => name,
                        Err(os_string) => {
                            eprintln!(
                                "Skipping file '{:?}' in '{}' due to invalid UTF-8 name",
                                os_string,
                                folder.display()
                            );
                            return None;
                        }
                    };
                    let metadata = match file_path.metadata() {
                        Ok(metadata) => metadata,
                        Err(err) => {
                            eprintln!(
                                "Skipping file '{}' in '{}' due to metadata error: {err}",
                                file_path.display(),
                                folder.display()
                            );
                            return None;
                        }
                    };

                    Some(VirtualFile {
                        name,
                        file_path,
                        size: metadata.len(),
                    })
                } else {
                    None
                }
            }
            Err(err) => {
                eprintln!(
                    "Failed to read directory entry in '{}': {err}",
                    folder.display()
                );
                None
            }
        })
        .collect::<Vec<VirtualFile>>();

    Ok(Files::from(files)?)
}
