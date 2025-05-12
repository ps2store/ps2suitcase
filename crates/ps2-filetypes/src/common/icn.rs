use crate::Color;

pub const ICN_MAGIC: u32 = 0x010000;
pub const TEXTURE_WIDTH: usize = 128;
pub const TEXTURE_HEIGHT: usize = 128;
pub const TEXTURE_SIZE: usize = TEXTURE_WIDTH * TEXTURE_HEIGHT;

#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub x: i16,
    pub y: i16,
    pub z: i16,
    pub w: u16,
}

impl Vertex {
    pub fn new(x: i16, y: i16, z: i16, w: u16) -> Self {
        Self { x, y, z, w }
    }
}

#[derive(Debug)]
pub struct Normal {
    pub x: i16,
    pub y: i16,
    pub z: i16,
    pub w: u16,
}

impl Normal {
    pub fn new(x: i16, y: i16, z: i16, w: u16) -> Self {
        Self { x, y, z, w }
    }
}

#[derive(Debug)]
pub struct UV {
    pub u: i16,
    pub v: i16,
}

impl UV {
    pub fn new(u: i16, v: i16) -> Self {
        Self { u, v }
    }
}

#[derive(Debug)]
pub struct IcnTexture {
    pub pixels: [u16; TEXTURE_SIZE],
}

pub type AnimationShape = Vec<Vertex>;

#[derive(Debug)]
pub struct Key {
    pub time: f32,
    pub value: f32,
}

#[derive(Debug)]
pub struct Frame {
    pub shape_id: u32,
    pub keys: Vec<Key>,
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
    pub animation_shape_count: u32,
    pub vertex_count: u32,
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
