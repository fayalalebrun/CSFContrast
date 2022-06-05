use std::rc::Rc;

use glium::{
    backend::Facade,
    implement_uniform_block,
    program::ComputeShader,
    texture::InternalFormat,
    uniform,
    uniforms::{self, UniformBuffer},
    Surface, Texture2d,
};

pub struct Fft {
    shader: Rc<ComputeShader>,
    texture: Option<FftTexture>,
}

impl Fft {
    pub fn new(facade: &dyn Facade) -> Self {
        Self {
            shader: Rc::new(
                ComputeShader::from_source(facade, include_str!("fft/comp.glsl")).unwrap(),
            ),
            texture: None,
        }
    }
    pub fn process_texture<'a>(
        &'a mut self,
        facade: &dyn Facade,
        texture: &Texture2d,
    ) -> &'a FftTexture {
        if self.texture.is_none()
            || self.texture.as_ref().unwrap().orig.dimensions() != texture.dimensions()
        {
            self.texture = Some(FftTexture::new(facade, self.shader.clone(), &texture));
        } else {
            texture.as_surface().fill(
                &self.texture.as_ref().unwrap().orig.as_surface(),
                uniforms::MagnifySamplerFilter::Nearest,
            );
        }

        self.texture.as_ref().unwrap()
    }

    // No idea what this function does.
    fn clz(x: u32) -> u32 {
        const LUT: [u32; 32] = [
            0, 31, 9, 30, 3, 8, 13, 29, 2, 5, 7, 21, 12, 24, 28, 19, 1, 10, 4, 14, 6, 22, 25, 20,
            11, 15, 23, 26, 16, 27, 17, 18,
        ];

        let x = x + 1;
        let x = x.next_power_of_two();

        return LUT[(x.wrapping_mul(0x076be629) >> 27) as usize];
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ImgInfo {
    input_width: i32,
    input_height: i32,
    output_width: i32,
    output_height: i32,
    logtwo_width: i32,
    logtwo_height: i32,
    clz_width: i32,
    clz_height: i32,
    no_of_channels: i32,
}

implement_uniform_block!(
    ImgInfo,
    input_width,
    input_height,
    output_width,
    output_height,
    logtwo_width,
    logtwo_height,
    clz_width,
    clz_height,
    no_of_channels,
);

pub struct FftTexture {
    fft: Rc<ComputeShader>,
    orig: Texture2d,
    real: Texture2d,
    imag: Texture2d,
    img_info: UniformBuffer<ImgInfo>,
}

impl FftTexture {
    fn new(facade: &dyn Facade, fft: Rc<ComputeShader>, orig: &Texture2d) -> Self {
        let (width, height) = orig.dimensions();
        let fft_dims = (width.next_power_of_two(), height.next_power_of_two());
        let clz = (Fft::clz(fft_dims.0) + 1, Fft::clz(fft_dims.1) + 1);
        let img_info = ImgInfo {
            input_width: width as i32,
            input_height: height as i32,
            output_width: fft_dims.0 as i32,
            output_height: fft_dims.1 as i32,
            logtwo_width: 32 - clz.0 as i32,
            logtwo_height: 32 - clz.1 as i32,
            clz_width: clz.0 as i32,
            clz_height: clz.1 as i32,
            no_of_channels: format_channels(&orig.get_internal_format().unwrap()) as i32,
        };

        let orig = Texture2d::empty_with_format(
            facade,
            glium::texture::UncompressedFloatFormat::F32F32F32F32,
            glium::texture::MipmapsOption::NoMipmap,
            width,
            height,
        )
        .unwrap();

        let real = Texture2d::empty_with_format(
            facade,
            glium::texture::UncompressedFloatFormat::F32F32F32F32,
            glium::texture::MipmapsOption::NoMipmap,
            fft_dims.0,
            fft_dims.1,
        )
        .unwrap();
        let imag = Texture2d::empty_with_format(
            facade,
            glium::texture::UncompressedFloatFormat::F32F32F32F32,
            glium::texture::MipmapsOption::NoMipmap,
            fft_dims.0,
            fft_dims.1,
        )
        .unwrap();

        let img_info = UniformBuffer::new(facade, img_info).unwrap();

        Self {
            fft,
            orig,
            real,
            imag,
            img_info,
        }
    }

    fn invoke(&self, stage: u32, work_groups: u32) {
        let input_unit = self
            .orig()
            .image_unit(uniforms::ImageUnitFormat::RGBA32F)
            .unwrap();

        let real_unit = self
            .real()
            .image_unit(uniforms::ImageUnitFormat::RGBA32F)
            .unwrap();

        let imag_unit = self
            .imag()
            .image_unit(uniforms::ImageUnitFormat::RGBA32F)
            .unwrap();

        self.fft.execute(
            uniform! {inputImage: input_unit, realPart: real_unit, imagPart: imag_unit, img_info: &self.img_info, stage: stage},
            work_groups,
            1,
            1,
        );
    }

    pub fn fft(&self, _facade: &dyn Facade) {
        self.invoke(0, self.img_info.read().unwrap().output_width as u32);
        self.invoke(1, self.img_info.read().unwrap().output_height as u32);
    }

    pub fn ifft(&self, _facade: &dyn Facade) {
        self.invoke(2, self.img_info.read().unwrap().output_height as u32);
        self.invoke(3, self.img_info.read().unwrap().output_width as u32);
    }

    pub fn orig<'b>(&'b self) -> &'b Texture2d {
        &self.orig
    }
    pub fn real<'b>(&'b self) -> &'b Texture2d {
        &self.real
    }
    pub fn imag<'b>(&'b self) -> &'b Texture2d {
        &self.imag
    }
}

fn format_channels(format: &InternalFormat) -> u32 {
    match format {
        InternalFormat::OneComponent { ty1: _, bits1: _ } => 1,
        InternalFormat::TwoComponents {
            ty1: _,
            bits1: _,
            ty2: _,
            bits2: _,
        } => 2,
        InternalFormat::ThreeComponents {
            ty1: _,
            bits1: _,
            ty2: _,
            bits2: _,
            ty3: _,
            bits3: _,
        } => 3,
        InternalFormat::FourComponents {
            ty1: _,
            bits1: _,
            ty2: _,
            bits2: _,
            ty3: _,
            bits3: _,
            ty4: _,
            bits4: _,
        } => 4,
    }
}
