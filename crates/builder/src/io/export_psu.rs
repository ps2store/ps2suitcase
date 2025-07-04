use ps2_filetypes::chrono::{DateTime, Utc};
use ps2_filetypes::{PSUEntry, PSUEntryKind, PSUWriter, DIR_ID, FILE_ID, PSU};
use std::fs::File;
use std::io::Write;
use crate::AppState;

pub fn export_psu(state: &mut AppState) -> std::io::Result<()> {
    let folder_name = state
        .opened_folder
        .clone()
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    let target_filename = folder_name.to_owned() + ".psu";

    if let Some(filename) = rfd::FileDialog::new()
        .set_file_name(target_filename)
        .save_file()
    {
        let mut psu = PSU::default();

        let root = PSUEntry {
            id: DIR_ID,
            size: state.files.len() as u32 + 2,
            created: Utc::now().naive_utc(),
            sector: 0,
            modified: Utc::now().naive_utc(),
            name: folder_name,
            kind: PSUEntryKind::Directory,
            contents: None,
        };
        let cur = PSUEntry {
            id: FILE_ID,
            size: 0,
            created: Utc::now().naive_utc(),
            sector: 0,
            modified: Utc::now().naive_utc(),
            name: ".".to_string(),
            kind: PSUEntryKind::File,
            contents: Some(vec![]),
        };
        let parent = PSUEntry {
            id: FILE_ID,
            size: 0,
            created: Utc::now().naive_utc(),
            sector: 0,
            modified: Utc::now().naive_utc(),
            name: "..".to_string(),
            kind: PSUEntryKind::File,
            contents: Some(vec![]),
        };

        psu.entries.push(root);
        psu.entries.push(cur);
        psu.entries.push(parent);

        for file in state.files.iter() {
            let metadata = file.file_path.metadata()?;
            let contents = std::fs::read(&file.file_path)?;
            let size = contents.len() as u32;

            let created_at: DateTime<Utc> = metadata.modified()?.into();
            let modified_at: DateTime<Utc> = metadata.modified()?.into();

            psu.entries.push(PSUEntry {
                id: FILE_ID,
                size,
                sector: 0,
                contents: Some(contents),
                name: file.name.clone(),
                created: created_at.naive_local(),
                modified: modified_at.naive_local(),
                kind: PSUEntryKind::File,
            });
        }
        let data = PSUWriter::new(psu).to_bytes()?;
        File::create(&filename)?.write_all(&data)?;
    }

    Ok(())
}
