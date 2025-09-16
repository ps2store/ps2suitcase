use eframe::glow;
use eframe::glow::HasContext;
use std::collections::HashMap;
use std::fmt;

pub enum UniformValueInternal {
    Int(i32),
    Float(f32),
    Matrix4f(cgmath::Matrix4<f32>),
    Vector2f(cgmath::Vector2<f32>),
    Vector3f(cgmath::Vector3<f32>),
    Vector4f(cgmath::Vector4<f32>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UniformType {
    Int,
    Float,
    Matrix4,
    Vector2,
    Vector3,
    Vector4,
    Sampler2D,
    Unknown(u32),
}

impl UniformType {
    fn from_gl_type(gl_type: u32) -> Self {
        match gl_type {
            glow::FLOAT => UniformType::Float,
            glow::FLOAT_MAT4 => UniformType::Matrix4,
            glow::FLOAT_VEC2 => UniformType::Vector2,
            glow::FLOAT_VEC3 => UniformType::Vector3,
            glow::FLOAT_VEC4 => UniformType::Vector4,
            glow::INT => UniformType::Int,
            glow::SAMPLER_2D => UniformType::Sampler2D,
            _ => UniformType::Unknown(gl_type),
        }
    }

    fn from_uniform_value(value: &UniformValueInternal) -> Self {
        match value {
            UniformValueInternal::Int(_) => UniformType::Int,
            UniformValueInternal::Float(_) => UniformType::Float,
            UniformValueInternal::Matrix4f(_) => UniformType::Matrix4,
            UniformValueInternal::Vector2f(_) => UniformType::Vector2,
            UniformValueInternal::Vector3f(_) => UniformType::Vector3,
            UniformValueInternal::Vector4f(_) => UniformType::Vector4,
        }
    }

    fn accepts(self, actual: UniformType) -> bool {
        match self {
            UniformType::Sampler2D => matches!(actual, UniformType::Int),
            UniformType::Unknown(_) => true,
            _ => self == actual,
        }
    }
}

impl fmt::Display for UniformType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UniformType::Int => write!(f, "int"),
            UniformType::Float => write!(f, "float"),
            UniformType::Matrix4 => write!(f, "mat4"),
            UniformType::Vector2 => write!(f, "vec2"),
            UniformType::Vector3 => write!(f, "vec3"),
            UniformType::Vector4 => write!(f, "vec4"),
            UniformType::Sampler2D => write!(f, "sampler2D"),
            UniformType::Unknown(gl_type) => write!(f, "unknown(0x{gl_type:04X})"),
        }
    }
}

#[derive(Clone, Debug)]
struct UniformMetadata {
    name: String,
    uniform_type: UniformType,
}

pub trait UniformValue {
    fn uniform_value(&self) -> UniformValueInternal;
}

impl UniformValue for i32 {
    fn uniform_value(&self) -> UniformValueInternal {
        UniformValueInternal::Int(*self)
    }
}

impl UniformValue for f32 {
    fn uniform_value(&self) -> UniformValueInternal {
        UniformValueInternal::Float(*self)
    }
}

impl UniformValue for cgmath::Matrix4<f32> {
    fn uniform_value(&self) -> UniformValueInternal {
        UniformValueInternal::Matrix4f(self.clone())
    }
}

impl UniformValue for cgmath::Vector4<f32> {
    fn uniform_value(&self) -> UniformValueInternal {
        UniformValueInternal::Vector4f(self.clone())
    }
}

impl UniformValue for cgmath::Vector3<f32> {
    fn uniform_value(&self) -> UniformValueInternal {
        UniformValueInternal::Vector3f(self.clone())
    }
}

impl UniformValue for cgmath::Vector2<f32> {
    fn uniform_value(&self) -> UniformValueInternal {
        UniformValueInternal::Vector2f(self.clone())
    }
}

#[derive(Clone, Debug)]
pub struct Program {
    program: glow::Program,
    uniforms: HashMap<String, UniformMetadata>,
}

impl Program {
    pub unsafe fn new(
        gl: &glow::Context,
        vertex_shader_source: &str,
        fragment_shader_source: &str,
    ) -> Self {
        let program = gl.create_program().expect("Cannot create program");

        let shader_sources = [
            (glow::VERTEX_SHADER, vertex_shader_source),
            (glow::FRAGMENT_SHADER, fragment_shader_source),
        ];

        let shaders: Vec<_> = shader_sources
            .iter()
            .map(|(shader_type, shader_source)| {
                let shader = gl
                    .create_shader(*shader_type)
                    .expect("Cannot create shader");
                gl.shader_source(shader, shader_source);
                gl.compile_shader(shader);
                assert!(
                    gl.get_shader_compile_status(shader),
                    "Shader compile error: {}",
                    gl.get_shader_info_log(shader)
                );
                gl.attach_shader(program, shader);
                shader
            })
            .collect();

        gl.link_program(program);
        assert!(
            gl.get_program_link_status(program),
            "Program link error: {}",
            gl.get_program_info_log(program)
        );

        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }

        let mut uniforms = HashMap::new();
        let active_uniforms = gl.get_active_uniforms(program);

        for index in 0..active_uniforms {
            if let Some(active_uniform) = gl.get_active_uniform(program, index) {
                let name = active_uniform.name;
                let uniform_type = UniformType::from_gl_type(active_uniform.utype);
                let metadata = UniformMetadata {
                    name: name.clone(),
                    uniform_type,
                };
                uniforms.insert(name, metadata);
            }
        }

        Self { program, uniforms }
    }

    pub fn gl(&self) -> glow::Program {
        self.program
    }

    pub fn get_attrib_location(&self, gl: &glow::Context, name: &str) -> Option<u32> {
        unsafe { gl.get_attrib_location(self.program, name) }
    }

    pub fn set(&self, gl: &glow::Context, name: &str, value: impl UniformValue) {
        unsafe {
            let program = self.program;
            let uniform_value = value.uniform_value();

            let Some(metadata) = self.uniform_metadata(name) else {
                eprintln!("Attempted to set unknown uniform `{name}`");
                return;
            };

            let actual_type = UniformType::from_uniform_value(&uniform_value);
            if !metadata.uniform_type.accepts(actual_type) {
                eprintln!(
                    "Type mismatch for uniform `{}` (requested as `{name}`): expected {}, found {}",
                    metadata.name, metadata.uniform_type, actual_type,
                );
                return;
            }

            let Some(location) = gl.get_uniform_location(self.program, name) else {
                eprintln!("Failed to get location `{name}`");
                return;
            };
            let location = Some(&location);

            match uniform_value {
                UniformValueInternal::Int(i) => {
                    gl.program_uniform_1_i32(program, location, i);
                }
                UniformValueInternal::Float(f) => {
                    gl.program_uniform_1_f32(program, location, f);
                }
                UniformValueInternal::Matrix4f(mat4) => {
                    gl.program_uniform_matrix_4_f32_slice(
                        program,
                        location,
                        false,
                        &convert_matrix(mat4),
                    );
                }
                UniformValueInternal::Vector2f(vec2) => {
                    gl.program_uniform_2_f32(program, location, vec2[0], vec2[1]);
                }
                UniformValueInternal::Vector3f(vec3) => {
                    gl.program_uniform_3_f32(program, location, vec3[0], vec3[1], vec3[2]);
                }
                UniformValueInternal::Vector4f(vec4) => {
                    gl.program_uniform_4_f32(program, location, vec4[0], vec4[1], vec4[2], vec4[3]);
                }
            }
        }
    }

    fn uniform_metadata(&self, name: &str) -> Option<&UniformMetadata> {
        self.uniforms.get(name).or_else(|| {
            canonical_uniform_name(name).and_then(|canonical| self.uniforms.get(&canonical))
        })
    }

    pub fn drop(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_program(self.program);
        }
    }
}

fn convert_matrix(mat: cgmath::Matrix4<f32>) -> Vec<f32> {
    vec![
        mat.x.x, mat.x.y, mat.x.z, mat.x.w, mat.y.x, mat.y.y, mat.y.z, mat.y.w, mat.z.x, mat.z.y,
        mat.z.z, mat.z.w, mat.w.x, mat.w.y, mat.w.z, mat.w.w,
    ]
}

fn canonical_uniform_name(name: &str) -> Option<String> {
    if !name.contains('[') {
        return None;
    }

    let mut canonical = String::with_capacity(name.len());
    let mut chars = name.chars().peekable();

    while let Some(ch) = chars.next() {
        canonical.push(ch);

        if ch == '[' {
            while let Some(next_ch) = chars.peek() {
                if *next_ch == ']' {
                    break;
                }
                chars.next();
            }

            canonical.push('0');

            match chars.next() {
                Some(']') => canonical.push(']'),
                Some(other) => canonical.push(other),
                None => return None,
            }
        }
    }

    Some(canonical)
}
