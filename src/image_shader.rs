use glium::{
    backend::Facade, implement_vertex, index::PrimitiveType, program::ProgramCreationInput,
    uniforms::Uniforms, IndexBuffer, Program, Surface, VertexBuffer,
};

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}

implement_vertex!(Vertex, position);

pub struct ImageShader {
    shader: Program,
    vertices: VertexBuffer<Vertex>,
    indices: IndexBuffer<u16>,
}

impl ImageShader {
    pub fn new(display: &dyn Facade, fragment_src: &str) -> Self {
        let vertices = {
            glium::VertexBuffer::new(
                display,
                &[
                    Vertex {
                        position: [-1.0, -1.0],
                    },
                    Vertex {
                        position: [-1.0, 1.0],
                    },
                    Vertex {
                        position: [1.0, 1.0],
                    },
                    Vertex {
                        position: [1.0, -1.0],
                    },
                ],
            )
            .unwrap()
        };

        let indices = glium::IndexBuffer::new(
            display,
            PrimitiveType::TrianglesList,
            &[0u16, 1, 2, 0, 2, 3],
        )
        .unwrap();

        // compiling shaders and linking them together
        let vertex_src = "
                    #version 300 es
                    in vec2 position;
                    out vec2 tex_coord;
                    void main() {
                        tex_coord = (position + 1.0)/2.0;
                        gl_Position = vec4(position, 0.0, 1.0);
                    }
                ";

        let creation_input = ProgramCreationInput::SourceCode {
            vertex_shader: vertex_src,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: None,
            fragment_shader: fragment_src,
            transform_feedback_varyings: None,
            outputs_srgb: false,
            uses_point_size: false,
        };

        let shader = Program::new(display, creation_input).unwrap();

        Self {
            vertices,
            indices,
            shader,
        }
    }

    pub fn draw<S, U>(&self, surface: &mut S, uniforms: &U)
    where
        S: Surface,
        U: Uniforms,
    {
        surface
            .draw(
                &self.vertices,
                &self.indices,
                &self.shader,
                uniforms,
                &Default::default(),
            )
            .unwrap();
    }
}
