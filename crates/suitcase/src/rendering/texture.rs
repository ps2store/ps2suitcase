use eframe::glow;
use eframe::glow::HasContext;

#[derive(Clone)]
pub struct Texture {
    texture: glow::Texture,
}

impl Texture {
    pub(crate) fn bind(&self, gl: &glow::Context) {
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
        }
    }
}

impl Texture {
    /**
     * Accepts RGBA textures only for now
     */
    pub fn new(gl: &glow::Context, data: &[u8]) -> Self {
        let texture = unsafe { gl.create_texture().unwrap() };
        let slf = Self { texture };

        slf.set(gl, data);

        slf
    }

    pub fn set(&self, gl: &glow::Context, data: &[u8]) {
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                128,
                128,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(data)),
            );
            gl.generate_mipmap(glow::TEXTURE_2D);
        }
    }

    pub fn drop(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_texture(self.texture);
        }
    }
}
