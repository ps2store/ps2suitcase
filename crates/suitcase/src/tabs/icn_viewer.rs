use crate::components::buttons::CustomButtons;
use crate::components::dialogs::Dialogs;
use crate::data::vec_math::ToGlow;
use crate::rendering::Shader;
use crate::tabs::Tab;
use crate::VirtualFile;
use cgmath::{point3, vec3, EuclideanSpace, Matrix4, Point3, Vector3};
use eframe::egui::{include_image, menu, Color32, Ui};
use eframe::glow::NativeTexture;
use eframe::{egui, egui_glow, glow};
use image::ImageReader;
use ps2_filetypes::color::Color;
use ps2_filetypes::{BinReader, BinWriter, ICNWriter, ICN};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug)]
struct OrbitCamera {
    pub target: Vector3<f32>,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub min_distance: f32,
    pub max_distance: f32,
}

impl OrbitCamera {
    pub fn update(&mut self, delta_yaw: f32, delta_pitch: f32, delta_zoom: f32) {
        self.yaw += delta_yaw;
        self.pitch += delta_pitch;

        // Clamp pitch to avoid gimbal lock
        self.pitch = self.pitch.clamp(-1.54, 1.54); // limit to ~88 degrees
        self.distance = (self.distance - delta_zoom).clamp(self.min_distance, self.max_distance);
    }

    pub fn position(&self) -> Vector3<f32> {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();
        Vector3::new(x, y, z) + self.target
    }

    pub fn view_matrix(&self) -> Matrix4<f32> {
        let position = self.position();
        Matrix4::look_at_rh(
            point3(position.x, position.y, position.z),
            Point3::from_vec(self.target),
            vec3(0.0, 1.0, 0.0),
        )
    }
}

pub struct ICNViewer {
    renderer: Arc<Mutex<Option<ICNRenderer>>>,
    file: String,
    path: PathBuf,
    camera: OrbitCamera,
    icn: ICN,
    dark_mode: bool,
    needs_update: bool,
    modified: bool,
    pub closing: bool,
}

impl ICNViewer {
    fn replace_texture(&mut self, path: PathBuf) {
        let image = ImageReader::open(&path).unwrap().decode().unwrap();

        let image = image.to_rgba8();
        self.icn.texture.pixels = image
            .pixels()
            .map(|p| Color::new(p.0[0], p.0[1], p.0[2], 255).into())
            .collect::<Vec<u16>>()
            .try_into()
            .unwrap();
        self.needs_update = true;
        self.modified = true;
        // eprintln!("{image:?}");
    }
}

impl ICNViewer {
    pub fn new(file: &VirtualFile) -> Self {
        let buf = std::fs::read(&file.file_path).expect("File not found");
        let icn = ps2_filetypes::ICNParser::read(&buf.clone()).unwrap();

        Self {
            renderer: Arc::new(Mutex::new(None)),
            file: file.name.clone(),
            path: file.file_path.clone(),
            camera: OrbitCamera {
                target: Vector3::new(0.0, 2.5, 0.0),
                distance: 10.0,
                yaw: 0.0,
                pitch: 0.0,
                min_distance: 1.0,
                max_distance: 50.0,
            },
            icn,
            dark_mode: true,
            needs_update: false,
            modified: false,
            closing: false,
        }
    }
    fn custom_painting(&mut self, ui: &mut Ui) {
        let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());
        let input = ui.input(|i| i.clone());

        let mut delta_yaw = 0.0;
        let mut delta_pitch = 0.0;
        let mut delta_zoom = 0.0;

        if input.pointer.primary_down() {
            let delta = input.pointer.delta();
            delta_yaw -= delta.x * 0.01;
            delta_pitch += delta.y * 0.01;
        }

        if input.raw_scroll_delta.y != 0.0 {
            delta_zoom += input.raw_scroll_delta.y * 0.1;
        }

        self.camera.update(delta_yaw, delta_pitch, delta_zoom);

        let renderer = self.renderer.clone();

        let icn = self.icn.clone();
        let needs_update = self.needs_update;
        let closing = self.closing;
        let camera = self.camera.clone();
        let aspect_ratio = rect.width() / rect.height();

        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(egui_glow::CallbackFn::new(move |_, painter| {
                let mut renderer = renderer.lock().unwrap();
                let renderer = renderer.get_or_insert_with(|| ICNRenderer::new(painter.gl(), &icn).expect("Failed to create ICNRenderer"));
                if needs_update {
                    renderer.replace_texture(painter.gl(), &icn);
                }
                if closing {
                    renderer.drop(painter.gl())
                } else {
                    renderer.paint(painter.gl(), aspect_ratio, camera);
                }
            })),
        };

        if needs_update {
            self.needs_update = false;
        }

        ui.painter().add(callback);
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            menu::bar(ui, |ui| {
                if ui
                    .icon_text_button(
                        include_image!("../../assets/icons/file-arrow-right.svg"),
                        "Export OBJ",
                    )
                    .clicked()
                {
                    if let Some(path) = ui.ctx().save_as(self.file.clone() + ".obj") {
                        File::create(path)
                            .unwrap()
                            .write_all(self.icn.export_obj().as_bytes())
                            .unwrap();
                    }
                }

                if ui
                    .icon_text_button(
                        include_image!("../../assets/icons/file-arrow-right.svg"),
                        "Export PNG",
                    )
                    .clicked()
                {
                    if let Some(path) = ui.ctx().save_as(self.file.clone() + ".png") {
                        File::create(path)
                            .unwrap()
                            .write_all(&self.icn.export_png())
                            .unwrap();
                    }
                }

                if ui
                    .icon_text_button(
                        include_image!("../../assets/icons/photo-plus.svg"),
                        "Replace Texture",
                    )
                    .clicked()
                {
                    if let Some(path) = ui.ctx().open_file_filter(&["png"]) {
                        self.replace_texture(path);
                    }
                }

                ui.checkbox(&mut self.dark_mode, "Dark Mode");
            });

            ui.centered_and_justified(|ui| {
                let fill = if self.dark_mode {
                    ui.style().visuals.window_fill
                } else {
                    Color32::from_rgb(0xAA, 0xAA, 0xAA)
                };
                egui::Frame::canvas(ui.style()).fill(fill).show(ui, |ui| {
                    self.custom_painting(ui);
                });
            });
        });
    }
}

impl Tab for ICNViewer {
    fn get_id(&self) -> &str {
        &self.file
    }

    fn get_title(&self) -> String {
        self.file.clone()
    }

    fn get_modified(&self) -> bool {
        self.modified
    }

    fn save(&mut self) {
        let mut file = File::create(&self.path).expect("Failed to create file");
        let bytes = ICNWriter::new(self.icn.clone())
            .write()
            .expect("Failed to save file");
        file.write_all(&bytes).expect("Failed to write to file");
        self.modified = false;
    }
}

#[derive(Clone)]
pub struct ICNRenderer {
    shader: Shader,
    model_vao: glow::VertexArray,
    model_vbo: glow::Buffer,
    lines_vao: glow::VertexArray,
    lines_vbo: glow::Buffer,
    grid_vao: glow::VertexArray,
    grid_vbo: glow::Buffer,
    vertex_count: usize,
    texture: NativeTexture,
    lines_shader: Shader,
}

impl ICNRenderer {
    pub fn new(gl: &glow::Context, icn: &ICN) -> Result<Self, String> {
        use glow::HasContext as _;

        unsafe {
            let model_shader = Shader::new(
                gl,
                include_str!("../shaders/icn.vsh"),
                include_str!("../shaders/icn.fsh"),
            );
            let lines_shader = Shader::new(
                gl,
                include_str!("../shaders/outline.vsh"),
                include_str!("../shaders/outline.fsh"),
            );

            let pixels: Vec<u8> = icn
                .texture
                .pixels
                .into_iter()
                .flat_map(|pixel| {
                    let color: Color = pixel.into();
                    let bytes: [u8; 4] = color.into();
                    [bytes[0], bytes[1], bytes[2], 255]
                })
                .collect();

            let vertices = icn.animation_shapes[0]
                .iter()
                .enumerate()
                .map(|(i, vertex)| {
                    let normal = icn.normals[i];
                    let uv = icn.uvs[i];
                    let color = icn.colors[i];
                    [
                        vertex.x as f32 / 4096.0,
                        -vertex.y as f32 / 4096.0,
                        -vertex.z as f32 / 4096.0,

                        color.r as f32 / 255.0,
                        color.g as f32 / 255.0,
                        color.b as f32 / 255.0,
                        color.a as f32 / 255.0,

                        normal.x as f32 / 4096.0,
                        -normal.y as f32 / 4096.0,
                        -normal.z as f32 / 4096.0,
                        uv.u as f32 / 4096.0,
                        uv.v as f32 / 4096.0,
                    ]
                })
                .flatten()
                .collect::<Vec<f32>>();

            let data = vertices.as_slice();

            let model_vao = gl
                .create_vertex_array()?;
            gl.bind_vertex_array(Some(model_vao));

            let model_vbo = gl.create_buffer()?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(model_vbo));
            let triangle_vertices_u8: &[u8] =
                core::slice::from_raw_parts(data.as_ptr() as *const u8, size_of_val(data));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, triangle_vertices_u8, glow::STATIC_DRAW);

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 12 * size_of::<f32>() as i32, 0);

            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(
                1,
                4,
                glow::FLOAT,
                false,
                12 * size_of::<f32>() as i32,
                3 * size_of::<f32>() as i32,
            );

            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(
                2,
                3,
                glow::FLOAT,
                false,
                12 * size_of::<f32>() as i32,
                7 * size_of::<f32>() as i32,
            );

            gl.enable_vertex_attrib_array(3);
            gl.vertex_attrib_pointer_f32(
                3,
                2,
                glow::FLOAT,
                false,
                12 * size_of::<f32>() as i32,
                10 * size_of::<f32>() as i32,
            );

            let texture = gl.create_texture()?;
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR_MIPMAP_LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );

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

            let lines_vao = gl
                .create_vertex_array()?;
            gl.bind_vertex_array(Some(lines_vao));

            let lines_vbo = gl.create_buffer()?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(lines_vbo));

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 3 * size_of::<f32>() as i32, 0);

            let vertices = generate_wireframe_box(vec3(6.0, 6.0, 6.0), vec3(0.0, 3.0, 0.0));

            let lines_vertices_u8: &[u8] =
                core::slice::from_raw_parts(vertices.as_ptr() as *const u8, size_of_val(&vertices[..]));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, lines_vertices_u8, glow::STATIC_DRAW);


            let grid_vao = gl.create_vertex_array()?;
            gl.bind_vertex_array(Some(grid_vao));

            let grid_vbo = gl.create_buffer()?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(grid_vbo));

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 3 * size_of::<f32>() as i32, 0);

            let vertices = generate_grid_lines(10, 1.0);
            let grid_vertices_u8: &[u8] =
                core::slice::from_raw_parts(vertices.as_ptr() as *const u8, size_of_val(&vertices[..]));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, grid_vertices_u8, glow::STATIC_DRAW);


            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.bind_vertex_array(None);


            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);

            Ok(Self {
                shader: model_shader,
                lines_shader,
                model_vao,
                model_vbo,
                lines_vao,
                lines_vbo,
                grid_vao,
                grid_vbo,
                vertex_count: icn.animation_shapes[0].len(),
                texture,
            })
        }
    }

    pub fn replace_texture(&mut self, gl: &glow::Context, icn: &ICN) {
        use glow::HasContext as _;

        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            let image = icn
                .texture
                .pixels
                .iter()
                .flat_map(|&color| {
                    let color: Color = color.into();
                    let bytes: [u8; 4] = color.into();
                    bytes
                })
                .collect::<Vec<u8>>();
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                128,
                128,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(image.as_slice())),
            );
            gl.generate_mipmap(glow::TEXTURE_2D);
        }
    }

    fn paint(&mut self, gl: &glow::Context, aspect_ratio: f32, orbit_camera: OrbitCamera) {
        use glow::HasContext as _;

        let projection = cgmath::perspective(cgmath::Deg(45.0), aspect_ratio, 0.1, 100.0);
        let view = orbit_camera.view_matrix();
        let model: Matrix4<f32> = Matrix4::from_translation(vec3(0.0, 0.0, 0.0));

        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);
            gl.clear(glow::DEPTH_BUFFER_BIT);

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

            gl.disable(glow::DEPTH_TEST);
            gl.polygon_mode(glow::FRONT_AND_BACK, glow::LINE);

            gl.uniform_3_f32_slice(Some(&gl.get_uniform_location(program, "color").unwrap()), &vec3::<f32>(0.0, 0.0, 0.0).to_glow());
            gl.bind_vertex_array(Some(self.grid_vao));
            gl.draw_arrays(glow::LINES, 0, 84);

            gl.polygon_mode(glow::FRONT_AND_BACK, glow::FILL);
            gl.enable(glow::DEPTH_TEST);


            gl.uniform_3_f32_slice(Some(&gl.get_uniform_location(program, "color").unwrap()), &vec3::<f32>(1.0, 0.0, 0.0).to_glow());
            gl.bind_vertex_array(Some(self.lines_vao));
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
            gl.bind_vertex_array(Some(self.model_vao));
            gl.draw_arrays(glow::TRIANGLES, 0, self.vertex_count as i32);
        }
    }

    pub fn drop(&self, gl: &glow::Context) {
        use glow::HasContext as _;
        unsafe {
            gl.delete_vertex_array(self.model_vao);
            gl.delete_buffer(self.lines_vbo);
            gl.delete_vertex_array(self.lines_vao);
            gl.delete_buffer(self.model_vbo);
            gl.delete_vertex_array(self.grid_vao);
            gl.delete_buffer(self.grid_vbo);
            self.shader.drop(gl);
            self.lines_shader.drop(gl);
            gl.delete_texture(self.texture);
        }
    }
}

fn convert_matrix(mat: Matrix4<f32>) -> Vec<f32> {
    vec![
        mat.x.x, mat.x.y, mat.x.z, mat.x.w, mat.y.x, mat.y.y, mat.y.z, mat.y.w, mat.z.x, mat.z.y,
        mat.z.z, mat.z.w, mat.w.x, mat.w.y, mat.w.z, mat.w.w,
    ]
}

pub fn generate_wireframe_box(size: Vector3<f32>, center: Vector3<f32>) -> Vec<f32> {
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
    [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0), // bottom rectangle
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4), // top rectangle
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7), // vertical lines
    ]
        .iter()
        .map(|(start, end)| {
            let a = corners[*start] + center;
            let b = corners[*end] + center;

            vec![a.x, a.y, a.z, b.x, b.y, b.z]
        })
        .flatten()
        .collect::<Vec<_>>()
}

fn generate_grid_lines(size: i32, step: f32) -> Vec<f32> {
    let mut lines = Vec::new();
    let half = size as f32 * step;

    for i in -size..=size {
        let p = i as f32 * step;

        lines.extend_from_slice(&[p, 0.0, -half, p, 0.0, half]); // Vertical lines
        lines.extend_from_slice(&[-half, 0.0, p, half, 0.0, p]); // Horizontal lines
    }

    lines
}