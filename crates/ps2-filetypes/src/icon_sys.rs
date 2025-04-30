use std::io::{Cursor, Read, Result};

use byteorder::{ReadBytesExt, LE};
use unicode_normalization::UnicodeNormalization;

use crate::util::parse_cstring;

#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Clone, Copy)]
pub struct ColorF {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[derive(Clone, Copy)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

pub struct IconSys {
    pub title_line_transparency: u16,
    pub background_transparency: u32,
    pub background_colors: [Color; 4],
    pub light_directions: [Vector; 3],
    pub light_colors: [Color; 3],
    pub ambient_color: ColorF,
    pub title: String,
    pub icon_file: String,
    pub icon_copy_file: String,
    pub icon_delete_file: String,
}

impl IconSys {
    pub fn new(bytes: Vec<u8>) -> Self {
        parse_icon_sys(bytes).unwrap()
    }
}

#[expect(unused)]
struct IconSysParser {
    c: Cursor<Vec<u8>>,
    len: usize,
}

fn parse_icon_sys(bytes: Vec<u8>) -> Result<IconSys> {
    let mut c = Cursor::new(bytes);

    let mut magic = vec![0u8; 4];
    c.read_exact(&mut magic)?;

    let title_line_transparency = c.read_u16::<LE>()?;
    _ = c.read_u16::<LE>()?;
    let background_transparency = c.read_u32::<LE>()?;
    _ = c.read_u32::<LE>();

    let background_colors = [
        parse_color(&mut c)?,
        parse_color(&mut c)?,
        parse_color(&mut c)?,
        parse_color(&mut c)?,
    ];

    let light_directions = [
        parse_direction(&mut c)?,
        parse_direction(&mut c)?,
        parse_direction(&mut c)?,
    ];

    let light_colors = [
        parse_color(&mut c)?,
        parse_color(&mut c)?,
        parse_color(&mut c)?,
    ];

    let ambient_color = parse_colorf(&mut c)?;

    let mut title_buf = vec![0u8; 68];
    c.read_exact(&mut title_buf)?;

    let mut icon_file_buf = vec![0u8; 64];
    c.read_exact(&mut icon_file_buf)?;
    let mut icon_copy_file_buf = vec![0u8; 64];
    c.read_exact(&mut icon_copy_file_buf)?;
    let mut icon_delete_file_buf = vec![0u8; 64];
    c.read_exact(&mut icon_delete_file_buf)?;

    Ok(IconSys {
        title_line_transparency,
        background_transparency,
        background_colors,
        light_directions,
        light_colors,
        ambient_color,
        title: parse_sjis_string(&title_buf),
        icon_file: parse_cstring(&icon_file_buf),
        icon_copy_file: parse_cstring(&icon_copy_file_buf),
        icon_delete_file: parse_cstring(&icon_delete_file_buf),
    })
}

fn parse_sjis_string(c: &[u8]) -> String {
    let title = encoding_rs::SHIFT_JIS.decode(c).0.to_string();

    parse_cstring(&title.nfkc().collect::<String>().as_bytes())
}

fn parse_color(c: &mut Cursor<Vec<u8>>) -> Result<Color> {
    let r = c.read_u32::<LE>()? as u8;
    let g = c.read_u32::<LE>()? as u8;
    let b = c.read_u32::<LE>()? as u8;
    let a = c.read_u32::<LE>()? as u8;

    Ok(Color { r, g, b, a })
}

fn parse_colorf(c: &mut Cursor<Vec<u8>>) -> Result<ColorF> {
    let r = c.read_f32::<LE>()?;
    let g = c.read_f32::<LE>()?;
    let b = c.read_f32::<LE>()?;
    let a = c.read_f32::<LE>()?;

    Ok(ColorF { r, g, b, a })
}

fn parse_direction(c: &mut Cursor<Vec<u8>>) -> Result<Vector> {
    let x = c.read_f32::<LE>()?;
    let y = c.read_f32::<LE>()?;
    let z = c.read_f32::<LE>()?;
    let w = c.read_f32::<LE>()?;

    Ok(Vector { x, y, z, w })
}
