use chrono::{DateTime, Local, NaiveDateTime};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub const DIR_ID: u16 = 0x8427;
pub const FILE_ID: u16 = 0x8497;

pub const PAGE_SIZE: u32 = 0x400;

#[derive(Default, Clone)]
pub struct PSU {
    pub entries: Vec<PSUEntry>,
}

impl PSU {
    pub fn add_defaults(&mut self, name: &str, timestamp: NaiveDateTime) {
        self.entries.push(PSUEntry {
            id: DIR_ID,
            size: 2, // the number of files in the psu, including . and ..
            created: timestamp,
            sector: 0,
            modified: timestamp,
            name: name.to_owned(),
            kind: PSUEntryKind::Directory,
            contents: None,
        });
        self.entries.push(PSUEntry {
            id: DIR_ID,
            size: 0,
            created: timestamp,
            sector: 0,
            modified: timestamp,
            name: ".".to_string(),
            kind: PSUEntryKind::Directory,
            contents: None,
        });
        self.entries.push(PSUEntry {
            id: DIR_ID,
            size: 0,
            created: timestamp,
            sector: 0,
            modified: timestamp,
            name: "..".to_string(),
            kind: PSUEntryKind::Directory,
            contents: None,
        });
    }

    pub fn add_file(&mut self, file: &String) -> Result<(), String> {
        fn convert_timestamp(time: SystemTime) -> NaiveDateTime {
            let duration = time.duration_since(UNIX_EPOCH).unwrap();
            let local =
                DateTime::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())
                    .unwrap()
                    .with_timezone(&Local)
                    .naive_local();

            local
        }

        if !fs::exists(&file).unwrap() {
            return Err("file doesn't exist".to_string());
        }

        let file_data = fs::read(file).unwrap();
        let metadata = fs::metadata(file).unwrap();
        let filename = Path::new(file)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        self.entries.push(PSUEntry {
            id: FILE_ID,
            size: file_data.len() as u32,
            created: convert_timestamp(metadata.created().unwrap()),
            sector: 0,
            modified: convert_timestamp(metadata.modified().unwrap()),
            name: filename,
            kind: PSUEntryKind::File,
            contents: Some(file_data),
        });

        // ensure the root directory's size matches the number of entries including . and ..
        self.entries[0].size += 1;

        Ok(())
    }

    pub fn remove_entry(&mut self, entry_name: &String) -> Result<(), String> {
        if self
            .entries
            .iter()
            .find(|e| e.name == *entry_name)
            .is_none()
        {
            return Err("Entry does not exist".to_string());
        }

        self.entries
            .retain(|entry| entry.name != entry_name.to_owned());

        // ensure the root directory's size matches the number of entries including . and ..
        self.entries[0].size -= 1;

        Ok(())
    }
}

impl Display for PSU {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut output = format!(
            "{:12}| {:16}| {:9}| {:25}| {:25}",
            "Type", "Name", "Size", "Created", "Modified"
        );
        output = format!("{}\n{:-<99}", output, "");
        for entry in self.entries.clone() {
            output = format!(
                "{}\n{:12}| {:16}| {:9}| {:25}| {:25}",
                output,
                entry.kind.to_string(),
                entry.name,
                entry.size,
                entry.created.format("%Y-%m-%d %H:%M:%S"),
                entry.modified.format("%Y-%m-%d %H:%M:%S"),
            );
        }
        write!(f, "{}", output)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum PSUEntryKind {
    Directory,
    File,
}

impl Display for PSUEntryKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            PSUEntryKind::Directory => write!(f, "directory"),
            PSUEntryKind::File => write!(f, "file"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PSUEntry {
    pub id: u16,
    pub size: u32,
    pub created: NaiveDateTime,
    pub sector: u16,
    pub modified: NaiveDateTime,
    pub name: String,
    pub kind: PSUEntryKind,
    pub contents: Option<Vec<u8>>,
}

pub(crate) struct PSUParser {
    pub(crate) c: Cursor<Vec<u8>>,
    pub(crate) len: u64,
}
