use glium::{backend::Facade, uniform, Surface};

use crate::image_shader::ImageShader;

pub struct Grating {
    image_shader: ImageShader,
    frequency: f32,
}

impl Grating {
    pub fn new(facade: &dyn Facade) -> Self {
        let fragment_src = include_str!("grating/frag.glsl");
        let image_shader = ImageShader::new(facade, fragment_src);
        Self {
            image_shader,
            frequency: 150.0,
        }
    }

    pub fn draw<S>(&mut self, surface: &mut S)
    where
        S: Surface,
    {
        self.image_shader.draw(
            surface,
            &uniform! {
            frequency: self.frequency},
        );
    }

    pub fn frequency(&self) -> f32 {
        self.frequency
    }

    pub fn frequency_mut<'a>(&'a mut self) -> &'a mut f32 {
        &mut self.frequency
    }
}
