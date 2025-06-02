use eframe::glow;
use eframe::glow::HasContext;

pub struct Shader {
    program: glow::Program,
}

impl Shader {
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

        Self { program }
    }

    pub fn program(&self) -> glow::Program {
        self.program
    }
    
    pub fn drop(&mut self, gl: &glow::Context) {
        unsafe {
            gl.delete_program(self.program);
        }
    }
}
