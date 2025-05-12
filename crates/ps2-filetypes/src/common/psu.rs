use std::io::Cursor;

pub const DIR_ID: u16 = 0x8427;
pub const FILE_ID: u16 = 0x8497;

pub const PAGE_SIZE: u32 = 0x400;


#[derive(Default)]
pub struct PSU {
    pub entries: Vec<PSUEntry>,
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
    pub kind: PSUEntryKind,
    pub contents: Option<Vec<u8>>,
}

pub(crate) struct PSUParser {
    pub(crate) c: Cursor<Vec<u8>>,
    pub(crate) len: u64,
}
