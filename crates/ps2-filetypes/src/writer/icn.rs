use crate::{
    BinWriter,
    Color,
    Frame,
    Normal,
    Vertex,
    ICN,
    ICN_MAGIC,
    UV
};
use byteorder::{WriteBytesExt, LE};
use std::io::ErrorKind;

pub struct ICNWriter {
    icn: ICN,
}

impl ICNWriter {
    pub fn new(icn: ICN) -> Self {
        Self {
            icn,
        }
    }

    fn write_header(&self) -> std::io::Result<Vec<u8>> {
        let mut header = vec![];
        header.write_u32::<LE>(ICN_MAGIC)?;
        header.write_u32::<LE>(self.icn.header.animation_shape_count)?;
        header.write_u32::<LE>(self.icn.header.texture_type)?;
        header.write_u32::<LE>(0)?; // Padding
        header.write_u32::<LE>(self.icn.header.vertex_count)?;

        Ok(header)
    }
    
    fn write_animation_shapes(&self) -> std::io::Result<Vec<u8>> {
        let mut data = vec![];
        
        for i in 0..self.icn.header.vertex_count {
            for j in 0..self.icn.header.animation_shape_count {
                data.extend(self.write_vertex(&self.icn.animation_shapes[j as usize][i as usize])?);
                data.extend(self.write_normal(&self.icn.normals[i as usize])?);
                data.extend(self.write_uv(&self.icn.uvs[i as usize])?);
                data.extend(self.write_color(&self.icn.colors[i as usize])?);
            }
        }
        
        Ok(data)
    }
    fn write_animation_data(&self) -> std::io::Result<Vec<u8>> {
        assert!(self.icn.header.vertex_count > 0);
        assert!(self.icn.header.animation_shape_count > 0);

        let mut data = vec![];
        data.write_u32::<LE>(0x01)?; // tag
        data.write_u32::<LE>(self.icn.animation_header.frame_length)?;
        data.write_f32::<LE>(self.icn.animation_header.anim_speed)?;
        data.write_u32::<LE>(self.icn.animation_header.play_offset)?;
        data.write_u32::<LE>(self.icn.animation_header.frame_count)?;
        
        for frame in self.icn.frames.iter() {
            data.extend(self.write_frame(frame)?);
        }
        
        Ok(data)
    }
    fn write_texture(&self) -> std::io::Result<Vec<u8>> {
        if self.icn.header.texture_type <= 0x07 {
            self.write_texture_uncompressed()
        } else {
            self.write_texture_compressed()
        }
    }
    
    fn write_texture_uncompressed(&self) -> std::io::Result<Vec<u8>> {
        let mut data = vec![];
        for pixel in self.icn.texture.pixels {
            data.write_u16::<LE>(pixel)?;
        }
        
        Ok(data)
    }
    
    fn write_texture_compressed(&self) -> std::io::Result<Vec<u8>> {
        Err(std::io::Error::new(ErrorKind::InvalidData, "Failed to compress texture"))
    }
    
    fn write_frame(&self, frame: &Frame) -> std::io::Result<Vec<u8>> {
        let mut data = vec![];
        data.write_u32::<LE>(frame.shape_id)?;
        data.write_u32::<LE>((frame.keys.len() + 1) as u32)?;
        data.write_u32::<LE>(0)?;
        data.write_u32::<LE>(0)?;
        
        for key in frame.keys.iter() {
            data.write_f32::<LE>(key.time)?;
            data.write_f32::<LE>(key.value)?;
        }
        
        Ok(data)
    }
    

    fn write_vertex(&self, vertex: &Vertex) -> std::io::Result<Vec<u8>> {
        let mut data = vec![];
        data.write_i16::<LE>(vertex.x)?;
        data.write_i16::<LE>(vertex.y)?;
        data.write_i16::<LE>(vertex.z)?;
        data.write_u16::<LE>(vertex.w)?;
        Ok(data)
    }

    fn write_normal(&self, normal: &Normal) -> std::io::Result<Vec<u8>> {
        let mut data = vec![];
        data.write_i16::<LE>(normal.x)?;
        data.write_i16::<LE>(normal.y)?;
        data.write_i16::<LE>(normal.z)?;
        data.write_u16::<LE>(normal.w)?;
        Ok(data)
    }

    fn write_uv(&self, uv: &UV) -> std::io::Result<Vec<u8>> {
        let mut data = vec![];
        data.write_i16::<LE>(uv.u)?;
        data.write_i16::<LE>(uv.v)?;
        Ok(data)
    }

    fn write_color(&self, color: &Color) -> std::io::Result<Vec<u8>> {
        let mut data = vec![];
        data.write_u8(color.r)?;
        data.write_u8(color.g)?;
        data.write_u8(color.b)?;
        data.write_u8(color.a)?;
        Ok(data)
    }
}

impl BinWriter for ICNWriter {
    fn write(&self) -> std::io::Result<Vec<u8>> {
        let mut file = vec![];
        
        file.extend(self.write_header()?);
        file.extend(self.write_animation_shapes()?);
        file.extend(self.write_animation_data()?);
        file.extend(self.write_texture()?);
        
        Ok(file)
    }
}