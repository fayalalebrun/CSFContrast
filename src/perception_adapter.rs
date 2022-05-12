use glium::{backend::Facade, uniform, Surface, Texture2d};

use crate::{csf::Csf, image_shader::ImageShader};

pub struct PerceptionAdapter {
    image_shader: ImageShader,
    pub target_cpd: f32,
}

impl PerceptionAdapter {
    pub fn new(facade: &dyn Facade) -> Self {
        let image_shader = ImageShader::new(facade, include_str!("perception_adapter/frag.glsl"));
        Self {
            image_shader,
            target_cpd: 2.0,
        }
    }

    pub fn draw<S>(&self, in_texture: &Texture2d, target: &mut S, csf: &Csf, current_cpd: f32)
    where
        S: Surface,
    {
        let scale_factor = csf.apply(self.target_cpd) / csf.apply(current_cpd);
        self.image_shader.draw(
            target,
            &uniform! {
                        in_texture: in_texture,
            scale_factor: scale_factor,
                    },
        )
    }
}
