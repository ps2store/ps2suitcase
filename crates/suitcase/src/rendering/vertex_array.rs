use crate::rendering::buffer::Buffer;
use crate::rendering::Program;
use eframe::glow;
use eframe::glow::HasContext;

#[derive(Debug, Default)]
pub struct Attributes {
    size: i32,
    attributes: Vec<(String, i32, i32)>,
}

pub fn attributes() -> Attributes {
    Attributes::default()
}

impl Attributes {
    pub fn float(mut self, name: &str, n: i32) -> Self {
        self.attributes.push((name.to_owned(), n, self.size));
        self.size += n * size_of::<f32>() as i32;

        self
    }
}

pub struct VertexArray {
    vertex_array: glow::VertexArray,
    program: Program,
    buffers: Vec<Buffer>,
    count: i32,
    pub type_: u32,
}

impl VertexArray {
    pub fn new<const N: usize>(
        gl: &glow::Context,
        program: &Program,
        buffer_map: [(Buffer, Attributes); N],
        type_: u32,
    ) -> Self {
        let (vertex_array, buffers, count) = unsafe {
            let vertex_array = gl.create_vertex_array().unwrap();
            let mut buffers = vec![];
            gl.bind_vertex_array(Some(vertex_array));

            let mut count = 0;

            for (buffer, vertex_format) in buffer_map {
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer.gl()));

                for (name, size, offset) in vertex_format.attributes {
                    let location = program
                        .get_attrib_location(gl, name.as_str())
                        .expect(format!("Can't find attribute {}", name).as_str());
                    gl.enable_vertex_attrib_array(location);
                    gl.vertex_attrib_pointer_f32(
                        location,
                        size,
                        glow::FLOAT,
                        false,
                        vertex_format.size,
                        offset,
                    );
                }
                count = buffer.size() / vertex_format.size;
                buffers.push(buffer);

                gl.bind_buffer(glow::ARRAY_BUFFER, None);
            }

            gl.bind_vertex_array(None);

            (vertex_array, buffers, count)
        };

        Self {
            program: program.clone(),
            vertex_array,
            buffers,
            count,
            type_,
        }
    }

    pub fn render(&self, gl: &glow::Context) {
        unsafe {
            gl.use_program(Some(self.program.gl()));
            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(self.type_, 0, self.count);
        }
    }

    pub fn drop(&self, gl: &glow::Context) {
        for vao in self.buffers.iter() {
            vao.drop(gl);
        }
        unsafe {
            gl.delete_vertex_array(self.vertex_array);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::rendering::vertex_array::{attributes, Attributes};

    #[test]
    fn test_vertex_buffer_layout() {
        let vertex_attributes = attributes()
            .float("vertex", 3)
            .float("color", 4)
            .float("normal", 3)
            .float("uv", 2);
        eprintln!("{vertex_attributes:?}")
    }
}
