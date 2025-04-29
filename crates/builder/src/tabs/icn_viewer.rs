use crate::tabs::Tab;
use crate::{AppState, VirtualFile};
use cgmath::{point3, vec3, Matrix4, Transform, Vector3};
use eframe::egui::{Ui, Vec2};
use eframe::glow::NativeTexture;
use eframe::{egui, egui_glow, glow};
use std::io::{Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex};
use cgmath::num_traits::FloatConst;
use crate::rendering::Shader;

pub struct ICNViewer {
    renderer: Arc<Mutex<Option<ICNRenderer>>>,
    bytes: Arc<Vec<u8>>,
    file: String,
    angle: f32,
}

impl ICNViewer {
    pub fn new(app: Arc<Mutex<AppState>>, file: Arc<Mutex<VirtualFile>>) -> Self {
        let file = file.clone();
        let mut file = file.lock().unwrap();
        let bytes = if let Some(file) = &mut file.file {
            let mut buf = Vec::new();
            file.seek(SeekFrom::Start(0)).unwrap();
            file.read_to_end(&mut buf).unwrap();
            buf
        } else {
            vec![]
        };
        Self {
            renderer: Arc::new(Mutex::new(None)),
            bytes: Arc::new(bytes),
            file: file.name.clone(),
            angle: f32::PI()/2.0,
        }
    }
    fn custom_painting(&mut self, ui: &mut Ui) {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(300.0), egui::Sense::drag());

        self.angle += response.drag_motion().x * 0.01;

        let renderer = self.renderer.clone();
        let bytes = self.bytes.clone();
        
        let angle = self.angle;

        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(egui_glow::CallbackFn::new(move |_, painter| {
                let mut renderer = renderer.lock().unwrap();
                let bytes = bytes.clone();
                let renderer =
                    renderer.get_or_insert_with(|| ICNRenderer::new(painter.gl(), bytes));
                renderer.paint(painter.gl(), angle);
            })),
        };

        ui.painter().add(callback);
    }
}

impl Tab for ICNViewer {
    fn get_title(&self) -> String {
        self.file.clone()
    }

    fn get_content(&mut self, ui: &mut Ui) {
        egui::Frame::canvas(ui.style()).show(ui, |ui| {
            self.custom_painting(ui);
        });
    }
}

struct ICNRenderer {
    shader: Shader,
    vertex_array: glow::VertexArray,
    lines_array: glow::VertexArray,
    vertex_count: usize,
    texture: NativeTexture,
    lines_shader: Shader,
}

impl ICNRenderer {
    pub fn new(gl: &glow::Context, bytes: Arc<Vec<u8>>) -> Self {
        use glow::HasContext as _;

        let icn = ps2_filetypes::ICN::new(bytes.clone().to_vec());

        unsafe {
            let shader = Shader::new(gl, include_str!("../shaders/icn.vsh"), include_str!("../shaders/icn.fsh"));
            let lines_shader = Shader::new(gl, include_str!("../shaders/outline.vsh"), include_str!("../shaders/outline.fsh"));

            let pixels: Vec<u8> = icn.texture.pixels.into_iter().flat_map(|pixel| {
                let r = pixel & 0x1f;
                let g = (pixel >> 5) & 0x1f;
                let b = (pixel >> 10) & 0x1f;
                let a = if pixel & 0x8000 != 0 { 255 } else { 0 };

                [(r * 255 / 31) as u8, (g * 255 / 31) as u8, (b * 255 / 31) as u8, a as u8]
            }).collect();

            let mut vertices = Vec::new();
            for (i, vertex) in icn.animation_shapes[0].iter().enumerate() {
                vertices.push(vertex.x as f32 / 4096.0);
                vertices.push(-vertex.y as f32 / 4096.0);
                vertices.push(-vertex.z as f32 / 4096.0);

                vertices.push(icn.normals[i].x as f32 / 4096.0);
                vertices.push(-icn.normals[i].y as f32 / 4096.0);
                vertices.push(-icn.normals[i].z as f32 / 4096.0);

                vertices.push(icn.uvs[i].u as f32 / 4096.0);
                vertices.push(icn.uvs[i].v as f32 / 4096.0);
            }

            let data = vertices.as_slice();


            let vertex_array = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");
            gl.bind_vertex_array(Some(vertex_array));

            let vbo = gl.create_buffer().expect("Cannot create buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let triangle_vertices_u8: &[u8] =
                core::slice::from_raw_parts(data.as_ptr() as *const u8, size_of_val(data));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, triangle_vertices_u8, glow::STATIC_DRAW);

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 8 * size_of::<f32>() as i32, 0);

            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(
                1,
                3,
                glow::FLOAT,
                false,
                8 * size_of::<f32>() as i32,
                3 * size_of::<f32>() as i32,
            );

            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(
                2,
                2,
                glow::FLOAT,
                false,
                8 * size_of::<f32>() as i32,
                6 * size_of::<f32>() as i32,
            );

            let texture = gl.create_texture().expect("Cannot create texture");
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR_MIPMAP_LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);

            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                128,
                128,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(pixels.as_slice())),
            );
            gl.generate_mipmap(glow::TEXTURE_2D);

            let lines_array = gl.create_vertex_array().expect("Cannot create vertex array");
            gl.bind_vertex_array(Some(lines_array));

            let vbo2 = gl.create_buffer().expect("Cannot create buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo2));

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 3 * size_of::<f32>() as i32, 0);

            let data: Vec<f32> = generate_wireframe_box(vec3(5.0, 5.0, 5.0));
            let data = data.as_slice();

            let lines_vertices_u8: &[u8] =
                core::slice::from_raw_parts(data.as_ptr() as *const u8, size_of_val(data));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, lines_vertices_u8, glow::STATIC_DRAW);

            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.bind_vertex_array(None);

            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);

            Self {
                shader,
                lines_shader,
                vertex_array,
                lines_array,
                vertex_count: icn.animation_shapes[0].len(),
                texture,
            }
        }
    }

    fn paint(&mut self, gl: &glow::Context, angle: f32) {
        use glow::HasContext as _;

        let projection = cgmath::perspective(cgmath::Deg(45.0), 1.0, 0.1, 100.0);
        let x = f32::cos(angle) * 10.0;
        let y = f32::sin(angle) * 10.0;

        let view: Matrix4<f32> = Matrix4::look_at_rh(
            point3(x, 0.0, y),
            point3(0.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
        );
        let model: Matrix4<f32> = Matrix4::from_translation(vec3(0.0, -2.5, 0.0));

        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);

            gl.depth_mask(false);
            let program = self.lines_shader.program();
            gl.use_program(Some(program));
            gl.uniform_matrix_4_f32_slice(
                Some(&gl.get_uniform_location(program, "projection").unwrap()),
                false,
                &convert_matrix(projection),
            );
            gl.uniform_matrix_4_f32_slice(
                Some(&gl.get_uniform_location(program, "view").unwrap()),
                false,
                &convert_matrix(view),
            );
            gl.bind_vertex_array(Some(self.lines_array));
            gl.draw_arrays(glow::LINES, 0, 24);

            let program = self.shader.program();
            gl.use_program(Some(program));
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            gl.uniform_matrix_4_f32_slice(
                Some(&gl.get_uniform_location(program, "projection").unwrap()),
                false,
                &convert_matrix(projection),
            );
            gl.uniform_matrix_4_f32_slice(
                Some(&gl.get_uniform_location(program, "view").unwrap()),
                false,
                &convert_matrix(view),
            );
            gl.uniform_matrix_4_f32_slice(
                Some(&gl.get_uniform_location(program, "model").unwrap()),
                false,
                &convert_matrix(model),
            );
            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(glow::TRIANGLES, 0, self.vertex_count as i32);

        }
    }
}

fn convert_matrix(mat: Matrix4<f32>) -> Vec<f32> {
    vec![
        mat.x.x, mat.x.y, mat.x.z, mat.x.w,
        mat.y.x, mat.y.y, mat.y.z, mat.y.w,
        mat.z.x, mat.z.y, mat.z.z, mat.z.w,
        mat.w.x, mat.w.y, mat.w.z, mat.w.w,
    ]
}

pub fn generate_wireframe_box(size: Vector3<f32>) -> Vec<f32> {
    let half = size * 0.5;

    let corners = [
        Vector3::new(-half.x, -half.y, -half.z),
        Vector3::new(half.x, -half.y, -half.z),
        Vector3::new(half.x, half.y, -half.z),
        Vector3::new(-half.x, half.y, -half.z),
        Vector3::new(-half.x, -half.y, half.z),
        Vector3::new(half.x, -half.y, half.z),
        Vector3::new(half.x, half.y, half.z),
        Vector3::new(-half.x, half.y, half.z),
    ];

    // Each pair of points forms a line
    let edges = [
        (0, 1), (1, 2), (2, 3), (3, 0), // bottom rectangle
        (4, 5), (5, 6), (6, 7), (7, 4), // top rectangle
        (0, 4), (1, 5), (2, 6), (3, 7), // vertical lines
    ];

    let mut vertices = Vec::new();

    for (start, end) in edges.iter() {
        let a = corners[*start];
        let b = corners[*end];

        vertices.push(a.x);
        vertices.push(a.y);
        vertices.push(a.z);

        vertices.push(b.x);
        vertices.push(b.y);
        vertices.push(b.z);
    }

    vertices
}