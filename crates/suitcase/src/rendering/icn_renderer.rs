use cgmath::{vec3, Matrix4, Vector3};
use eframe::glow;
use ps2_filetypes::color::Color;
use ps2_filetypes::{Key, ICN};
use crate::rendering::animation::Timeline;
use crate::rendering::buffer::Buffer;
use crate::rendering::orbit_camera::OrbitCamera;
use crate::rendering::program::Program;
use crate::rendering::texture::Texture;
use crate::rendering::vertex_array::{attributes, VertexArray};

pub struct ICNRenderer {
    model_shader: Program,
    model: VertexArray,
    model_texture: Texture,
    lines: VertexArray,
    grid: VertexArray,
    lines_shader: Program,
    shapes: Vec<Vec<f32>>,
    timelines: Vec<Timeline>,
}

impl ICNRenderer {
    pub fn new(gl: &glow::Context, icn: &ICN) -> Result<Self, String> {
        unsafe {
            let model_shader = Program::new(
                gl,
                include_str!("../shaders/icn.vsh"),
                include_str!("../shaders/icn.fsh"),
            );
            let lines_shader = Program::new(
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

            let mut shapes = vec![];
            let mut timelines = vec![];

            for (i, frame) in icn.frames.iter().enumerate() {
                let mut keys = frame.keys.clone();
                if i == 0 {
                    keys.insert(0, Key{time: 0.0, value: 1.0});
                }
                timelines.push(Timeline::new(keys))
            }

            for shape in icn.animation_shapes.iter() {
                let vertices = shape
                    .iter()
                    .flat_map(|vertex|
                        [
                            vertex.x as f32 / 4096.0,
                            -vertex.y as f32 / 4096.0,
                            -vertex.z as f32 / 4096.0,
                        ]
                    )
                    .collect::<Vec<f32>>();
                shapes.push(vertices);
            }

            let data = (0..icn.animation_shapes[0].len())
                .flat_map(|i| {
                    let color = icn.colors[i];
                    let normal = icn.normals[i];
                    let uv = icn.uvs[i];
                    [
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
                .collect::<Vec<_>>();

            let model = VertexArray::new(
                gl,
                &model_shader,
                [(
                    Buffer::new(gl, &shapes[0]),
                    attributes()
                        .float("position", 3)
                ), (
                    Buffer::new(gl, &data),
                    attributes()
                        .float("color", 4)
                        .float("normal", 3)
                        .float("uv", 2),
                    )],
                glow::TRIANGLES,
            );

            let grid = VertexArray::new(
                gl,
                &lines_shader,
                [(
                    Buffer::new(gl, &generate_grid_lines(10, 1.0)),
                    attributes().float("position", 3),
                )],
                glow::LINES,
            );
            let vertices = generate_wireframe_box(vec3(6.0, 6.0, 6.0), vec3(0.0, 3.0, 0.0));

            let lines = VertexArray::new(
                gl,
                &lines_shader,
                [(
                    Buffer::new(gl, &vertices),
                    attributes().float("position", 3),
                )],
                glow::LINES,
            );

            let model_texture = Texture::new(gl, &pixels);

            Ok(Self {
                shapes,
                model_shader,
                lines_shader,
                model,
                lines,
                grid,
                model_texture,
                timelines,
            })
        }
    }

    pub fn replace_texture(&mut self, gl: &glow::Context, icn: &ICN) {
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
        self.model_texture.set(gl, &image);
    }

    pub fn paint(&mut self, gl: &glow::Context, aspect_ratio: f32, orbit_camera: OrbitCamera, frame: u32) {
        use glow::HasContext as _;

        let projection = cgmath::perspective(cgmath::Deg(45.0), aspect_ratio, 0.1, 100.0);
        let view = orbit_camera.view_matrix();
        let model: Matrix4<f32> = Matrix4::from_translation(vec3(0.0, 0.0, 0.0));


        let mut vertices = vec![0f32; self.shapes[0].len()];

        let mut sum = 0.0;
        let mut weights = vec![];

        for timeline in self.timelines.iter() {
            let y = timeline.evaluate(frame as f32);
            sum += y;
            weights.push(y);
        }

        if !self.timelines.is_empty() {
            for (j, shape) in self.shapes.iter().enumerate() {
                for (i, vertex) in shape.iter().enumerate() {
                    vertices[i] += *vertex * weights[j]/sum;
                }
            }
            self.model.buffer(0).set(gl, &vertices);
        }


        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);
            gl.clear(glow::DEPTH_BUFFER_BIT);

            self.lines_shader.set(gl, "projection", projection);
            self.lines_shader.set(gl, "view", view);
            self.lines_shader.set(gl, "color", vec3(0.0, 0.0, 0.0));

            gl.disable(glow::DEPTH_TEST);
            gl.polygon_mode(glow::FRONT_AND_BACK, glow::LINE);
            self.grid.render(gl);
            gl.polygon_mode(glow::FRONT_AND_BACK, glow::FILL);
            gl.enable(glow::DEPTH_TEST);

            self.lines_shader.set(gl, "color", vec3(1.0, 0.0, 0.0));
            self.lines.render(gl);

            self.model_texture.bind(gl);
            self.model_shader.set(gl, "tex", 0);
            self.model_shader.set(gl, "projection", projection);
            self.model_shader.set(gl, "view", view);
            self.model_shader.set(gl, "model", model);
            self.model.render(gl);
        }
    }

    pub fn drop(&self, gl: &glow::Context) {
        self.lines.drop(gl);
        self.grid.drop(gl);
        self.model.drop(gl);
        self.model_shader.drop(gl);
        self.lines_shader.drop(gl);
        self.model_texture.drop(gl);
    }
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
