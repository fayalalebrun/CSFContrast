use glium::{
    backend::Facade,
    implement_uniform_block,
    program::ComputeShader,
    texture::InternalFormat,
    uniform,
    uniforms::{self, UniformBuffer},
    Texture2d,
};

pub struct Fft {
    shader: ComputeShader,
}

impl Fft {
    pub fn new(facade: &dyn Facade) -> Self {
        Self {
            shader: ComputeShader::from_source(facade, include_str!("fft/comp.glsl")).unwrap(),
        }
    }
    pub fn shader<'a>(&'a self) -> &'a ComputeShader {
        &self.shader
    }
    pub fn process_texture<'a>(
        &'a self,
        facade: &dyn Facade,
        texture: Texture2d,
    ) -> FftTexture<'a> {
        FftTexture::new(facade, self, texture)
    }

    // No idea what this function does.
    fn clz(x: u32) -> u32 {
        const LUT: [u32; 32] = [
            0, 31, 9, 30, 3, 8, 13, 29, 2, 5, 7, 21, 12, 24, 28, 19, 1, 10, 4, 14, 6, 22, 25, 20,
            11, 15, 23, 26, 16, 27, 17, 18,
        ];

        let x = x + 1;
        let x = x.next_power_of_two();

        return LUT[(x * 0x076be629 >> 27) as usize];
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ImgInfo {
    input_width: u32,
    input_height: u32,
    output_width: u32,
    output_height: u32,
    logtwo_width: u32,
    logtwo_height: u32,
    clz_width: u32,
    clz_height: u32,
    no_of_channels: u32,
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

pub struct FftTexture<'a> {
    fft: &'a Fft,
    orig: Texture2d,
    real: Texture2d,
    imag: Texture2d,
    img_info: UniformBuffer<ImgInfo>,
}

impl<'a> FftTexture<'a> {
    fn new(facade: &dyn Facade, fft: &'a Fft, orig: Texture2d) -> Self {
        let (width, height) = orig.dimensions();
        let fft_dims = (width.next_power_of_two(), height.next_power_of_two());
        let clz = (Fft::clz(fft_dims.0) + 1, Fft::clz(fft_dims.1) + 1);
        let img_info = ImgInfo {
            input_width: width,
            input_height: height,
            output_width: fft_dims.0,
            output_height: fft_dims.1,
            logtwo_width: 32 - clz.0,
            logtwo_height: 32 - clz.1,
            clz_width: clz.0,
            clz_height: clz.1,
            no_of_channels: format_channels(&orig.get_internal_format().unwrap()),
        };
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

        let img_info = UniformBuffer::immutable(facade, img_info).unwrap();

        Self {
            fft,
            orig,
            real,
            imag,
            img_info,
        }
    }

    fn invoke(&self, stage: u32, work_groups: u32) {
        self.fft.shader().execute(
            uniform! {inputImage: self.orig.sampled(), realPart: &self.real, imagePart: &self.imag, img_info: &self.img_info, stage: stage},
            work_groups,
            0,
            0,
        );
    }

    pub fn fft(&self, _facade: &dyn Facade) {
        self.invoke(0, self.img_info.read().unwrap().output_width);
        self.invoke(1, self.img_info.read().unwrap().output_height);
    }

    pub fn ifft(&self, _facade: &dyn Facade) {
        self.invoke(2, self.img_info.read().unwrap().output_height);
        self.invoke(3, self.img_info.read().unwrap().output_width);
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
