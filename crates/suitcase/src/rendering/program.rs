use eframe::glow;
use eframe::glow::HasContext;

pub enum UniformValueInternal {
    Int(i32),
    Float(f32),
    Matrix4f(cgmath::Matrix4<f32>),
    Vector2f(cgmath::Vector2<f32>),
    Vector3f(cgmath::Vector3<f32>),
    Vector4f(cgmath::Vector4<f32>),
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

        // TODO: Determine if we need type checking
        // let active_uniforms = gl.get_active_uniforms(program);
        //
        // for i in 0..active_uniforms {
        //     let ActiveUniform{name, size, utype} = gl.get_active_uniform(program, i).unwrap();
        //     println!("{} {} {}", name, size, utype);
        // }


        Self { program }
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
            let location = gl.get_uniform_location(self.program, name).expect(format!("Failed to get location {}", name).as_str());
            let location = Some(&location);

            match value.uniform_value() {
                UniformValueInternal::Int(i) => {
                    gl.program_uniform_1_i32(program, location, i);
                }
                UniformValueInternal::Float(f) => {
                    gl.program_uniform_1_f32(program, location, f);
                }
                UniformValueInternal::Matrix4f(mat4) => {
                    gl.program_uniform_matrix_4_f32_slice(program, location, false, &convert_matrix(mat4));
                }
                UniformValueInternal::Vector2f(vec2) => {
                    gl.program_uniform_2_f32(program, location, vec2[0], vec2[1]);
                }
                UniformValueInternal::Vector3f(vec3) => {
                    gl.program_uniform_3_f32(program, location, vec3[0], vec3[1], vec3[2]);
                }
                UniformValueInternal::Vector4f(vec4) => {
                    gl.program_uniform_4_f32(program, location, vec4[0], vec4[1], vec4[2], vec4[3]);
                },
            }
        }
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