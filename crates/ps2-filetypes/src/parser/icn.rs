use crate::color::Color;
use crate::{
    AnimationHeader, AnimationShape, BinReader, Frame, ICNHeader, IcnTexture, Key, Normal, Vertex,
    ICN, ICN_MAGIC, TEXTURE_SIZE, UV,
};
use byteorder::{ReadBytesExt, LE};
use image::codecs::png::PngEncoder;
use image::{EncodableLayout, RgbaImage};
use std::io::Cursor;

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

    pub fn export_png(&self) -> Vec<u8> {
        let mut png_data = Vec::new();
        let mut img = RgbaImage::new(128, 128);

        for (i, pixel) in img.pixels_mut().enumerate() {
            let color: Color = self.texture.pixels[i].into();

            pixel.0 = color.into();
        }

        let encoder = PngEncoder::new(&mut png_data);
        img.write_with_encoder(encoder)
            .expect("Failed to write PNG data");
        png_data
    }
}

pub struct ICNParser {
    c: Cursor<Vec<u8>>,
}

impl BinReader<ICN> for ICNParser {
    fn read(data: &[u8]) -> std::io::Result<ICN> {
        let mut parser = ICNParser {
            c: Cursor::new(data.to_vec()),
        };
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
}

impl ICNParser {
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

        for _ in 0..icnheader.animation_shape_count as usize {
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
        if texture_type & 0b0100 > 0 {
            if texture_type & 0b1000 > 0 {
                self.parse_texture_compressed()
            } else {
                self.parse_texture_uncompressed()
            }
        } else {
            Ok(IcnTexture {
                pixels: [0xFFFF; TEXTURE_SIZE],
            })
        }
    }

    fn parse_texture_uncompressed(&mut self) -> std::io::Result<IcnTexture> {
        let mut pixels: [u16; TEXTURE_SIZE] = [0; TEXTURE_SIZE];
        self.c.read_u16_into::<LE>(&mut pixels)?;

        Ok(IcnTexture { pixels })
    }

    fn parse_texture_compressed(&mut self) -> std::io::Result<IcnTexture> {
        let compressed_size = self.c.read_u32::<LE>()? as usize;
        let mut compressed = vec![0u16; compressed_size / 2];
        self.c.read_u16_into::<LE>(&mut compressed)?;

        let mut pixels: Vec<u16> = Vec::with_capacity(TEXTURE_SIZE);

        let mut offset = 0;
        while offset < compressed.len() {
            let rle_code = compressed[offset];
            offset += 1;

            if rle_code & 0x8000 != 0 {
                // Literal run
                let next_count = (0x8000 - (rle_code ^ 0x8000)) as usize;
                for _ in 0..next_count {
                    if offset >= compressed.len() || pixels.len() >= TEXTURE_SIZE {
                        break;
                    }
                    pixels.push(compressed[offset]);
                    offset += 1;
                }
            } else {
                // Repeated run
                let times = rle_code as usize;
                if times > 0 && offset < compressed.len() {
                    let pixel = compressed[offset];
                    offset += 1;
                    for _ in 0..times {
                        if pixels.len() >= TEXTURE_SIZE {
                            break;
                        }
                        pixels.push(pixel);
                    }
                }
            }
        }

        // Fill remaining pixels with 0 if decompressed data is smaller than TEXTURE_SIZE
        pixels.resize(TEXTURE_SIZE, 0);

        let mut final_pixels = [0u16; TEXTURE_SIZE];
        final_pixels.copy_from_slice(&pixels[..TEXTURE_SIZE]);

        Ok(IcnTexture {
            pixels: final_pixels,
        })
    }

    fn parse_frame(&mut self) -> std::io::Result<Frame> {
        let shape_id = self.c.read_u32::<LE>()?;
        let key_count = self.c.read_u32::<LE>()?;
        let mut keys = vec![];

        for _ in 0..key_count {
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
