use glium::{backend::Facade, program::ComputeShader, uniform, Texture2d};

pub struct ColorSpace {
    shader: ComputeShader,
}

impl ColorSpace {
    pub fn new(facade: &dyn Facade) -> Self {
        Self {
            shader: ComputeShader::from_source(facade, include_str!("color_space/comp.glsl"))
                .unwrap(),
        }
    }

    // Transforms the given texture from the RGB colorspace to the YCbCr BT.709 colorspace
    // Expects a texture in RGBA32F format
    pub fn rgb_to_ycbcr(&self, texture: &Texture2d) {
        self.invoke(texture, 0)
    }

    pub fn ycbcr_to_rgb(&self, texture: &Texture2d) {
        self.invoke(texture, 1)
    }

    fn invoke(&self, texture: &Texture2d, mode: u32) {
        let image_unit = texture
            .image_unit(glium::uniforms::ImageUnitFormat::RGBA32F)
            .unwrap();
        self.shader.execute(
            uniform! {
                image: image_unit,
            mode: mode,
                    },
            texture.width() / 64 + 1,
            texture.height(),
            1,
        )
    }
}
