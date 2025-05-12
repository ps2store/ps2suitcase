#![allow(dead_code, unused_variables)]

use std::io::{Cursor, Result};

pub struct MCD {}

impl MCD {
    pub fn new() -> Self {
        Self {}
    }
}

struct MCDParser {
    c: Cursor<Vec<u8>>,
    len: usize,
}

impl MCDParser {
    fn new(bytes: Vec<u8>) -> Self {
        let len = bytes.len();
        Self {
            c: Cursor::new(bytes),
            len,
        }
    }
    fn parse(bytes: Vec<u8>) -> Result<MCD> {
        let parser = MCDParser::new(bytes);

        Ok(MCD {})
    }
}
