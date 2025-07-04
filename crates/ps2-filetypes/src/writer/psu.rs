use crate::{PSUEntry, FILE_ID, PAGE_SIZE, PSU};
use byteorder::{WriteBytesExt, LE};
use chrono::{Datelike, NaiveDateTime, Timelike};
use std::io::Write;

pub struct PSUWriter {
    psu: PSU
}

impl PSUWriter {
    pub fn new(psu: PSU) -> Self {
        Self { psu }
    }
    
    fn write_timestamp(&self, timestamp: NaiveDateTime) -> std::io::Result<Vec<u8>> {
        let mut data = vec![];
        data.write_u8(0)?;
        data.write_u8(timestamp.second() as u8)?;
        data.write_u8(timestamp.minute() as u8)?;
        data.write_u8(timestamp.hour() as u8)?;
        data.write_u8(timestamp.day() as u8)?;
        data.write_u8(timestamp.month0() as u8)?;
        data.write_u16::<LE>(timestamp.year() as u16)?; 
        
        Ok(data)
    }
    
    fn write_string(&self, string: String) -> std::io::Result<Vec<u8>> {
        let remainder = 448 - string.len();
        let mut data = vec![];
        for c in string.chars() {
            data.push(c as u8);
        }
        data.extend(vec![0; remainder]);

        Ok(data) 
    }
    
    fn write_entry(&self, entry: &PSUEntry) -> std::io::Result<Vec<u8>> {
        let mut data: Vec<u8> = vec![];
        data.write_u16::<LE>(entry.id)?;
        data.write_u16::<LE>(0)?;
        data.write_u32::<LE>(entry.size)?;
        data.write_all(&self.write_timestamp(entry.created)?)?;
        data.write_u16::<LE>(entry.sector)?;
        data.write_u16::<LE>(0)?;
        data.write_u32::<LE>(0)?;
        data.write_all(&self.write_timestamp(entry.modified)?)?;
        let padding = vec![0u8; 32];
        data.write_all(&padding)?;
        data.write_all(&self.write_string(entry.name.clone())?)?;

        if entry.id == FILE_ID {
            data.write_all(&entry.contents.clone().unwrap())?;
            let rem = 1024 - (entry.size % 1024);

            let rem = if rem == PAGE_SIZE { 0 } else { rem as i64 };
            for _ in 0..rem {
                data.write_u8(0)?;
            }
        }

        Ok(data) 
    }

    pub fn to_bytes(&self) -> std::io::Result<Vec<u8>> {
        let mut data = vec![];

        for entry in &self.psu.entries {
            data.extend(self.write_entry(entry)?);
        }

        Ok(data)
    }
}