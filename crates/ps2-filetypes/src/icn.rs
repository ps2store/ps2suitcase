use crate::util::parse_cstring;
use crate::Color;
use byteorder::{ReadBytesExt, LE};
use std::fmt::{format, Write};
use std::io::{Cursor, Read};

const ICN_MAGIC: u32 = 0x010000;

const TEXTURE_WIDTH: usize = 128;
const TEXTURE_HEIGHT: usize = 128;
pub const TEXTURE_SIZE: usize = TEXTURE_WIDTH * TEXTURE_HEIGHT;

#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub x: i16,
    pub y: i16,
    pub z: i16,
    pub w: u16,
}

#[derive(Debug)]
pub struct Normal {
    pub x: i16,
    pub y: i16,
    pub z: i16,
    pub w: u16,
}

#[derive(Debug)]
pub struct UV {
    pub u: i16,
    pub v: i16,
}
#[derive(Debug)]
pub struct IcnTexture {
    pub pixels: [u16; TEXTURE_SIZE],
}

type AnimationShape = Vec<Vertex>;

#[derive(Debug)]
pub struct Key {
    pub time: f32,
    pub value: f32,
}

#[derive(Debug)]
pub struct Frame {
    shape_id: u32,
    keys: Vec<Key>,
}

#[derive(Debug)]
pub struct AnimationHeader {
    pub tag: u32,
    pub frame_length: u32,
    pub anim_speed: f32,
    pub play_offset: u32,
    pub frame_count: u32,
}

#[derive(Debug)]
pub struct ICNHeader {
    animation_shape_count: u32,
    vertex_count: u32,
    pub texture_type: u32,
}

#[derive(Debug)]
pub struct ICN {
    pub header: ICNHeader,
    pub animation_shapes: Vec<AnimationShape>,
    pub normals: Vec<Normal>,
    pub uvs: Vec<UV>,
    pub colors: Vec<Color>,
    pub texture: IcnTexture,
    pub animation_header: AnimationHeader,
    pub frames: Vec<Frame>,
}

impl ICN {
    pub fn export_obj(&self) -> String {
        let mut output = String::new();
        let shape = self.animation_shapes[0].clone();

        output += "mtllib list.mtl\no list\n";

        for v in shape {
            output += format!(
                "v {} {} {}\n",
                v.x as f32 / 4096.0,
                -v.y as f32 / 4096.0,
                -v.z as f32 / 4096.0
            )
            .as_str();
        }

        for i in 0..self.header.vertex_count as usize {
            output += format!(
                "vt {} {}\n",
                self.uvs[i].u as f32 / 4096.0,
                1.0 - (self.uvs[i].v as f32 / 4096.0)
            )
            .as_str();
        }
        output += "usemtl tex\n";
        for f in 0..self.header.vertex_count / 3 {
            output += format!(
                "f {}/{} {}/{} {}/{}\n",
                f * 3 + 1,
                f * 3 + 1,
                f * 3 + 1 + 1,
                f * 3 + 1 + 1,
                f * 3 + 2 + 1,
                f * 3 + 2 + 1,
            )
            .as_str();
        }

        output
    }
}

impl ICN {
    pub fn new(bytes: Vec<u8>) -> Self {
        ICNParser::parse(bytes).unwrap()
    }
}

struct ICNParser {
    c: Cursor<Vec<u8>>,
    len: usize,
}

impl ICNParser {
    pub fn new(bytes: Vec<u8>) -> Self {
        let len = bytes.len();
        ICNParser {
            c: Cursor::new(bytes),
            len,
        }
    }
    pub fn parse(bytes: Vec<u8>) -> std::io::Result<ICN> {
        let mut parser = ICNParser::new(bytes);
        let header = parser.parse_header().expect("Failed to parse ICN header");
        let (animation_shapes, normals, uvs, colors) = parser
            .parse_animation_shapes(&header)
            .expect("Failed to parse animation shapes");
        let (animation_header, frames) = parser
            .parse_animation_data()
            .expect("Failed to parse animation header");
        let texture = parser
            .parse_texture(header.texture_type)
            .expect("Failed to parse texture");

        Ok(ICN {
            header,
            animation_shapes,
            normals,
            uvs,
            colors,
            animation_header,
            frames,
            texture,
        })
    }

    pub fn parse_header(&mut self) -> std::io::Result<ICNHeader> {
        let magic = self.c.read_u32::<LE>()?;
        assert_eq!(magic, ICN_MAGIC);
        let animation_shape_count = self.c.read_u32::<LE>()?;
        let texture_type = self.c.read_u32::<LE>()?;
        _ = self.c.read_u32::<LE>()?;
        let vertex_count = self.c.read_u32::<LE>()?;

        Ok(ICNHeader {
            animation_shape_count,
            texture_type,
            vertex_count,
        })
    }

    fn parse_animation_shapes(
        &mut self,
        icnheader: &ICNHeader,
    ) -> std::io::Result<(Vec<AnimationShape>, Vec<Normal>, Vec<UV>, Vec<Color>)> {
        let mut shapes: Vec<AnimationShape> =
            Vec::with_capacity(icnheader.animation_shape_count as usize);
        let mut normals: Vec<Normal> = Vec::with_capacity(icnheader.vertex_count as usize);
        let mut uvs: Vec<UV> = Vec::with_capacity(icnheader.vertex_count as usize);
        let mut colors: Vec<Color> = Vec::with_capacity(icnheader.vertex_count as usize);

        for i in 0..icnheader.animation_shape_count as usize {
            shapes.push(vec![
                Vertex {
                    x: 0,
                    y: 0,
                    z: 0,
                    w: 0
                };
                icnheader.vertex_count as usize
            ]);
        }
        for i in 0..icnheader.vertex_count as usize {
            for j in 0..icnheader.animation_shape_count as usize {
                shapes[j][i] = self.read_vertex()?;
            }
            normals.push(self.read_normal()?);
            uvs.push(self.read_uv()?);
            colors.push(self.read_color()?);
        }

        Ok((shapes, normals, uvs, colors))
    }

    fn parse_animation_data(&mut self) -> std::io::Result<(AnimationHeader, Vec<Frame>)> {
        let tag = self.c.read_u32::<LE>()?;
        let frame_length = self.c.read_u32::<LE>()?;
        let anim_speed = self.c.read_f32::<LE>()?;
        let play_offset = self.c.read_u32::<LE>()?;
        let frame_count = self.c.read_u32::<LE>()?;
        let mut frames = vec![];

        assert_eq!(tag, 0x01);

        for _ in 0..frame_count {
            frames.push(self.parse_frame()?);
        }

        Ok((
            AnimationHeader {
                tag,
                frame_length,
                anim_speed,
                play_offset,
                frame_count,
            },
            frames,
        ))
    }

    fn parse_texture(&mut self, texture_type: u32) -> std::io::Result<IcnTexture> {
        if texture_type <= 0x07 {
            self.parse_texture_uncompressed()
        } else {
            self.parse_texture_compressed()
        }
    }

    fn parse_texture_uncompressed(&mut self) -> std::io::Result<IcnTexture> {
        let mut pixels: [u16; TEXTURE_SIZE] = [0; TEXTURE_SIZE];
        self.c.read_u16_into::<LE>(&mut pixels)?;

        Ok(IcnTexture { pixels })
    }

    fn parse_texture_compressed(&mut self) -> std::io::Result<IcnTexture> {
        let size = self.c.read_u32::<LE>()? as usize / 2;
        let mut compressed = vec![0; size];
        self.c.read_u16_into::<LE>(&mut compressed)?;

        let mut pixels: [u16; TEXTURE_SIZE] = [0; TEXTURE_SIZE];

        let mut index = 0;
        let mut offset = 0;

        while offset < size {
            let rep_count = compressed[offset];
            offset += 1;
            if rep_count < 0xff00 {
                let pixel = compressed[offset];
                offset += 1;
                for _ in 0..rep_count {
                    if index >= TEXTURE_SIZE {
                        break;
                    }
                    pixels[index] = pixel;
                    index += 1;
                }
            } else {
                let actual_count = 0xffff ^ rep_count;
                for _ in 0..=actual_count {
                    if index >= TEXTURE_SIZE {
                        break;
                    }
                    let pixel = compressed[offset];
                    offset += 1;
                    pixels[index] = pixel;
                    index += 1;
                }
            }
        }

        Ok(IcnTexture { pixels })
    }

    fn parse_frame(&mut self) -> std::io::Result<Frame> {
        let shape_id = self.c.read_u32::<LE>()?;
        let key_count = self.c.read_u32::<LE>()?;
        _ = self.c.read_u32::<LE>()?;
        _ = self.c.read_u32::<LE>()?;
        let mut keys = vec![];

        for _ in 0..key_count - 1 {
            let time = self.c.read_f32::<LE>()?;
            let value = self.c.read_f32::<LE>()?;

            keys.push(Key { time, value });
        }

        Ok(Frame { shape_id, keys })
    }

    fn read_vertex(&mut self) -> std::io::Result<Vertex> {
        let x = self.c.read_i16::<LE>()?;
        let y = self.c.read_i16::<LE>()?;
        let z = self.c.read_i16::<LE>()?;
        let w = self.c.read_u16::<LE>()?;
        Ok(Vertex { x, y, z, w })
    }
    fn read_normal(&mut self) -> std::io::Result<Normal> {
        let x = self.c.read_i16::<LE>()?;
        let y = self.c.read_i16::<LE>()?;
        let z = self.c.read_i16::<LE>()?;
        let w = self.c.read_u16::<LE>()?;
        Ok(Normal { x, y, z, w })
    }

    fn read_uv(&mut self) -> std::io::Result<UV> {
        let u = self.c.read_i16::<LE>()?;
        let v = self.c.read_i16::<LE>()?;
        Ok(UV { u, v })
    }
    fn read_color(&mut self) -> std::io::Result<Color> {
        let r = self.c.read_u8()?;
        let b = self.c.read_u8()?;
        let g = self.c.read_u8()?;
        let a = self.c.read_u8()?;
        Ok(Color { r, g, b, a })
    }
}
