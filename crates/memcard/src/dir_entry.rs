use std::io;
use std::io::{Cursor, Read, Seek};
use byteorder::{ReadBytesExt, LE};

pub const DF_READ: u16 = 0x0001;
pub const DF_WRITE: u16 = 0x0002;
pub const DF_EXECUTE: u16 = 0x0004;
pub const DF_PROTECTED: u16 = 0x0008;
pub const DF_FILE: u16 = 0x0010;
pub const DF_DIRECTORY: u16 = 0x0020;
pub const DF_0400: u16 =  0x0400;
pub const DF_EXISTS: u16 = 0x8000;
pub const DF_HIDDEN: u16 = 0x2000;

#[derive(Debug, Copy, Clone)]
pub struct DateTime {
    seconds: u8,
    minutes: u8,
    hours: u8,
    days: u8,
    months: u8,
    years: u16,
}

impl DateTime {
    fn from_bytes(bytes: &[u8]) -> DateTime {
        let seconds = bytes[1];
        let minutes = bytes[2];
        let hours = bytes[3];
        let days = bytes[4];
        let months = bytes[5];
        let years = u16::from_le_bytes([bytes[6], bytes[7]]);

        Self {
            seconds,
            minutes,
            hours,
            days,
            months,
            years,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct DirEntry {
    pub(crate) mode: u16,
    pub(crate) length: u32,
    pub(crate) created: DateTime,
    pub cluster: u32,
    dir_entry: u32,
    modified: DateTime,
    attributes: u32,
    pub(crate) name: [u8; 32],
}

impl DirEntry {
    pub(crate) fn from_bytes(bytes: &[u8]) -> io::Result<DirEntry> {
        let mut c = Cursor::new(bytes);
        let mode = c.read_u16::<LE>()?;
        let _ = c.read_u16::<LE>()?;
        let length = c.read_u32::<LE>()?;

        let mut created = [0; 8];
        c.read_exact(&mut created)?;
        let created = DateTime::from_bytes(&created);

        let cluster = c.read_u32::<LE>()?;
        let dir_entry = c.read_u32::<LE>()?;

        let mut modified = [0; 8];
        c.read_exact(&mut modified)?;
        let modified = DateTime::from_bytes(&modified);

        let attributes = c.read_u32::<LE>()?;
        c.seek_relative(28)?;
        let mut name = [0; 32];
        c.read_exact(&mut name)?;


        Ok(DirEntry {
            mode,
            length,
            created,
            cluster,
            dir_entry,
            modified,
            attributes,
            name,
        })
    }
    pub fn is_empty(&self) -> bool {
        self.name[0] == 0x00
    }

    pub fn is_deleted(&self) -> bool {
        self.name[0] == 0xE5
    }

    pub fn name_as_string(&self) -> String {
        // Convert Shift-JIS or ASCII depending on your card
        String::from_utf8_lossy(&self.name)
            .trim_end_matches('\0')
            .to_string()
    }

    pub fn is_directory(&self) -> bool {
        self.mode & DF_DIRECTORY != 0 // Typical DOS attribute flag for directory
    }
}