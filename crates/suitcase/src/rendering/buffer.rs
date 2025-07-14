use eframe::glow;
use eframe::glow::HasContext;

pub struct Buffer {
    buffer: glow::Buffer,
    size: i32,
}

impl Buffer {
    pub fn new(gl: &glow::Context, data: &[f32]) -> Self {
        let buffer = unsafe { gl.create_buffer().expect("Create Buffer") };

        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, core::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * size_of::<f32>()), glow::STATIC_DRAW);
        }

        Buffer { buffer, size: (data.len() * size_of::<f32>())  as i32 }
    }

    pub fn set(&self, gl: &glow::Context, data: &[f32]) {
        assert_eq!(data.len(), self.size as usize);
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.buffer));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, core::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * size_of::<f32>()), glow::STATIC_DRAW);
        }
    }

    pub fn size(&self) -> i32 {
        self.size
    }
    pub fn gl(&self) -> glow::Buffer {
        self.buffer
    }

    pub fn drop(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_buffer(self.buffer);
        }
    }
}
