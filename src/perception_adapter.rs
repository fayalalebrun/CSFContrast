use glium::{
    backend::Facade, implement_uniform_block, program::ComputeShader, uniform,
    uniforms::UniformBuffer, Texture2d,
};

use crate::csf::Csf;

pub struct PerceptionAdapter {
    shader: ComputeShader,
    csf_upload: CsfUpload,
}

impl PerceptionAdapter {
    pub fn new(facade: &dyn Facade) -> Self {
        let shader =
            ComputeShader::from_source(facade, include_str!("perception_adapter/comp.glsl"))
                .unwrap();
        Self {
            shader,
            csf_upload: CsfUpload::new(facade),
        }
    }

    pub fn draw(
        &mut self,
        facade: &dyn Facade,
        real_texture: &Texture2d,
        imag_texture: &Texture2d,
        pixels_per_visual_degree: f32,
        csf: &Csf,
        target_pixels_per_visual_degree: f32,
    ) {
        use glium::uniforms::ImageUnitFormat::RGBA32F;
        let real_unit = real_texture.image_unit(RGBA32F).unwrap();
        let imag_unit = imag_texture.image_unit(RGBA32F).unwrap();
        let csf_lut = self.csf_upload.ubuffer(facade, csf);
        self.shader.execute(
            uniform! {
                                    realPart: real_unit,
                        imagPart: imag_unit,
                    pixels_per_visual_degree: pixels_per_visual_degree,
            target_pixels_per_visual_degree: target_pixels_per_visual_degree,
                CsfLut: csf_lut,

                                },
            real_texture.width() / 64 + 1,
            real_texture.height(),
            1,
        )
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CsfLut {
    lut_lower_limit: f32,
    lut_upper_limit: f32,
    lut_array: [f32; 4096],
}

impl CsfLut {
    pub fn from_csf(csf: &Csf) -> Self {
        let lut_lower_limit = 0.;
        let lut_upper_limit = 50.;
        let points = csf
            .plot_points(lut_lower_limit, lut_upper_limit, 4096)
            .map(|(_, y)| y);
        Self {
            lut_lower_limit,
            lut_upper_limit,
            lut_array: points.collect::<Vec<_>>().try_into().unwrap(),
        }
    }
}

implement_uniform_block!(CsfLut, lut_lower_limit, lut_upper_limit, lut_array,);

struct CsfUpload {
    ubuffer: UniformBuffer<CsfLut>,
    cached_csf: Csf,
}

impl CsfUpload {
    pub fn new(facade: &dyn Facade) -> Self {
        let ubuffer = UniformBuffer::empty(facade).unwrap();
        Self {
            ubuffer,
            cached_csf: Csf {
                a: 0.,
                ω: 0.,
                σ: 0.,
                k: 0.,
            },
        }
    }

    pub fn ubuffer<'a>(&'a mut self, facade: &dyn Facade, csf: &Csf) -> &'a UniformBuffer<CsfLut> {
        if self.cached_csf != *csf {
            self.cached_csf = csf.clone();
            self.ubuffer = UniformBuffer::immutable(facade, CsfLut::from_csf(csf)).unwrap();
        }
        &self.ubuffer
    }
}
