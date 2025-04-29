use byteorder::{WriteBytesExt, LE};
use chrono::{DateTime, Datelike, Timelike, Utc};
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::Path;

const DIR_ID: u16 = 0x8427;
const FILE_ID: u16 = 0x8497;

const PAGE_SIZE: u32 = 0x400;

fn timestamp_to_bytes(timestamp: chrono::NaiveDateTime) -> Result<Vec<u8>, std::io::Error> {
    let mut data: Vec<u8> = vec![];
    data.write_u8(0)?;
    data.write_u8(timestamp.second() as u8)?;
    data.write_u8(timestamp.minute() as u8)?;
    data.write_u8(timestamp.hour() as u8)?;
    data.write_u8(timestamp.day() as u8)?;
    data.write_u8(timestamp.month0() as u8)?;
    data.write_u16::<LE>(timestamp.year() as u16)?;

    Ok(data)
}

fn encode_string(string: String) -> Result<Vec<u8>, std::io::Error> {
    let remainder = 448 - string.len();
    let mut data = vec![];
    for c in string.chars() {
        data.push(c as u8);
    }

    for _ in 0..remainder {
        data.push(0);
    }

    Ok(data)
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum PSUEntryKind {
    Directory,
    File,
}

#[derive(Debug, Clone)]
pub struct PSUEntry {
    pub id: u16,
    pub size: u32,
    pub created: chrono::NaiveDateTime,
    pub sector: u16,
    pub modified: chrono::NaiveDateTime,
    pub name: String,
    pub contents: Option<Vec<u8>>,
}

impl PSUEntry {
    pub fn new(name: String, file: &mut File) -> Result<Self, std::io::Error> {
        let metadata = file.metadata()?;
        let mut contents = vec![0u8; metadata.len() as usize];
        let size = file.read(&mut contents)? as u32;

        let created_at: DateTime<Utc> = metadata.modified()?.into();
        let modified_at: DateTime<Utc> = metadata.modified()?.into();

        Ok(Self {
            id: FILE_ID,
            size,
            sector: 0,
            contents: Some(contents),
            name,
            created: created_at.naive_local(),
            modified: modified_at.naive_local(),
        })
    }
    pub(crate) fn as_bytes(&self) -> Result<Vec<u8>, std::io::Error> {
        let mut data: Vec<u8> = vec![];
        data.write_u16::<LE>(self.id)?;
        data.write_u16::<LE>(0)?;
        data.write_u32::<LE>(self.size)?;
        data.write_all(&timestamp_to_bytes(self.created)?)?;
        data.write_u16::<LE>(self.sector)?;
        data.write_u16::<LE>(0)?;
        data.write_u32::<LE>(0)?;
        data.write_all(&timestamp_to_bytes(self.modified)?)?;
        let padding = vec![0u8; 32];
        data.write_all(&padding)?;
        data.write_all(&encode_string(self.name.clone())?)?;

        if (self.id == FILE_ID) {
            data.write_all(&self.contents.clone().unwrap())?;
            let rem = 1024 - (self.size % 1024);

            let rem = if rem == PAGE_SIZE { 0 } else { rem as i64 };
            for _ in 0..rem {
                data.write_u8(0)?;
            }
        }

        Ok(data)
    }
}

pub struct PSU {
    entries: Vec<PSUEntry>,
}

impl PSU {
    pub fn new() -> Self {
        Self { entries: vec![] }
    }

    pub fn add_file(&mut self, name: String, file: &mut File) -> Result<&mut Self, std::io::Error> {
        self.entries.push(PSUEntry::new(name, file)?);

        Ok(self)
    }

    pub fn write(&self, name: &Path) -> Result<(), std::io::Error> {
        let mut file = File::create(name)?;
        let root = PSUEntry {
            id: DIR_ID,
            size: self.entries.len() as u32 + 2,
            created: Utc::now().naive_utc(),
            sector: 0,
            modified: Utc::now().naive_utc(),
            name: name.to_str().unwrap().to_string(),
            contents: None,
        };
        let cur = PSUEntry {
            id: DIR_ID,
            size: 0,
            created: Utc::now().naive_utc(),
            sector: 0,
            modified: Utc::now().naive_utc(),
            name: ".".to_string(),
            contents: None,
        };
        let parent = PSUEntry {
            id: DIR_ID,
            size: 0,
            created: Utc::now().naive_utc(),
            sector: 0,
            modified: Utc::now().naive_utc(),
            name: "..".to_string(),
            contents: None,
        };
        file.write_all(root.as_bytes()?.as_slice())?;
        file.write_all(cur.as_bytes()?.as_slice())?;
        file.write_all(parent.as_bytes()?.as_slice())?;
        for entry in &self.entries {
            file.write_all(entry.as_bytes()?.as_slice())?;
        }

        Ok(())
    }
}
