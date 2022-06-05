use cgmath::Vector2;
use egui::plot::{Line, Plot, Value, Values};
use glium::{
    backend::Facade, framebuffer::SimpleFrameBuffer, texture::SrgbTexture2d, Display, Frame,
    Surface, Texture2d,
};

use crate::{
    color_space::ColorSpace,
    csf::Csf,
    fft::Fft,
    grating::Grating,
    gstreamer::{CtxInfo, Gstreamer},
    perception_adapter::PerceptionAdapter,
};

pub struct System {
    grating: Grating,
    intermediate: Option<Texture2d>,
    flowers: SrgbTexture2d,
    screen_dims_mm: Vector2<f32>,
    screen_distance_mm: f32,
    target_distance_mm: f32,
    csf: Csf,
    adapter: PerceptionAdapter,
    adapt: bool,
    fft: Fft,
    color_space: ColorSpace,
    gstreamer: Gstreamer,
}

impl System {
    pub fn new(facade: &Display, initial_uri: &str) -> Self {
        let image = image::load(
            std::io::Cursor::new(&include_bytes!("../flowers.png")[..]),
            image::ImageFormat::Png,
        )
        .unwrap()
        .to_rgba8();
        let image_dimensions = image.dimensions();
        let image =
            glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
        let flowers = glium::texture::SrgbTexture2d::new(facade, image).unwrap();

        let grating = Grating::new(facade);

        let ctx_info = CtxInfo::new(crate::gstreamer::SurfaceType::Display(facade));
        let gstreamer = Gstreamer::new(facade, &ctx_info);
        gstreamer.set_uri(initial_uri);
        Self {
            grating,
            intermediate: None,
            flowers,
            screen_dims_mm: (600.0f32, 336.0f32).into(),
            screen_distance_mm: 750.,
            target_distance_mm: 900.,
            csf: Csf {
                a: 1.787,
                ω: 7.22,
                σ: 2.2,
                k: 0.71,
            },
            adapter: PerceptionAdapter::new(facade),
            adapt: true,
            fft: Fft::new(facade),
            color_space: ColorSpace::new(facade),
            gstreamer,
        }
    }

    pub fn draw(&mut self, facade: &dyn Facade, surface: &mut Frame) {
        if self.intermediate.is_none() {
            self.intermediate = Some(
                Texture2d::empty_with_format(
                    facade,
                    glium::texture::UncompressedFloatFormat::U16U16U16U16,
                    glium::texture::MipmapsOption::NoMipmap,
                    1920,
                    1080,
                )
                .unwrap(),
            );
        }
        let intermediate = self.intermediate.as_ref().unwrap();
        let mut int_surface = intermediate.as_surface();
        //self.grating.draw(&mut int_surface);
        // let flowers_fb = SimpleFrameBuffer::new(facade, &self.flowers).unwrap();
        // flowers_fb.fill(&int_surface, glium::uniforms::MagnifySamplerFilter::Nearest);
        let gstreamer_fb = self.gstreamer.draw(facade).as_surface();
        gstreamer_fb.fill(&int_surface, glium::uniforms::MagnifySamplerFilter::Nearest);

        if self.adapt {
            let pixels_per_vd =
                self.pixels_per_vd(intermediate.width() as f32, self.screen_distance_mm);
            let target_pixels_per_vd =
                self.pixels_per_vd(intermediate.width() as f32, self.target_distance_mm);

            let fft_tex = self.fft.process_texture(facade, intermediate);
            self.color_space.rgb_to_ycbcr(fft_tex.orig());
            fft_tex.fft(facade);
            self.adapter.draw(
                facade,
                fft_tex.real(),
                fft_tex.imag(),
                pixels_per_vd,
                &self.csf,
                target_pixels_per_vd,
            );
            fft_tex.ifft(facade);
            self.color_space.ycbcr_to_rgb(fft_tex.orig());

            fft_tex
                .orig()
                .as_surface()
                .fill(surface, glium::uniforms::MagnifySamplerFilter::Nearest);
        } else {
            int_surface.fill(surface, glium::uniforms::MagnifySamplerFilter::Nearest);
        }
    }

    fn pixels_per_vd(&self, pixels: f32, distance_mm: f32) -> f32 {
        pixels / self.total_visual_angle(distance_mm).to_degrees()
    }

    fn total_visual_angle(&self, distance: f32) -> f32 {
        2. * (self.screen_dims_mm.x / distance).atan()
    }

    pub fn draw_ui(&mut self, egui_ctx: &egui::Context) {
        egui::SidePanel::left("my_side_panel").show(egui_ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("# of cycles");
                ui.add(
                    egui::DragValue::new(self.grating.frequency_mut())
                        .speed(0.1)
                        .clamp_range(1.0..=1000.),
                )
            });
            ui.label("Screen dimensions: ");
            ui.horizontal(|ui| {
                ui.label("width:");
                ui.add(
                    egui::DragValue::new(&mut self.screen_dims_mm.x)
                        .speed(0.1)
                        .clamp_range(0.0..=100000.),
                );
                ui.label("mm");
            });
            ui.horizontal(|ui| {
                ui.label("height:");
                ui.add(
                    egui::DragValue::new(&mut self.screen_dims_mm.y)
                        .speed(0.1)
                        .clamp_range(0.0..=100000.),
                );
                ui.label("mm");
            });
            ui.horizontal(|ui| {
                ui.label("Distance to screen:");
                ui.add(
                    egui::DragValue::new(&mut self.screen_distance_mm)
                        .speed(0.1)
                        .clamp_range(0.0..=100000.),
                );
                ui.label("mm");
            });
            ui.label(format!(
                "Total visual angle: {}°",
                self.total_visual_angle(self.screen_distance_mm)
                    .to_degrees()
            ));
            if let Some(intermediate) = self.intermediate.as_ref() {
                ui.label(format!(
                    "Pixels per visual degree: {}",
                    self.pixels_per_vd(intermediate.width() as f32, self.screen_distance_mm),
                ));
                ui.label(format!(
                    "Target pixels per visual degree: {}",
                    self.pixels_per_vd(intermediate.width() as f32, self.target_distance_mm),
                ));
            }

            ui.heading("CSF");
            ui.horizontal(|ui| {
                ui.label("A");
                ui.add(egui::DragValue::new(&mut self.csf.a).speed(0.1));
                ui.label("k");
                ui.add(egui::DragValue::new(&mut self.csf.k).speed(0.1));
                ui.label("ω");
                ui.add(egui::DragValue::new(&mut self.csf.ω).speed(0.1));
                ui.label("σ");
                ui.add(egui::DragValue::new(&mut self.csf.σ).speed(0.1));
            });
            Plot::new("CSF plot")
                .view_aspect(2.0)
                .legend(Default::default())
                .show(ui, |plot_ui| {
                    plot_ui.line(self.plot_csf());
                });
            ui.heading("Adjustment Algorithm");
            ui.checkbox(&mut self.adapt, "Activate");
            ui.horizontal(|ui| {
                ui.label("Target distance to screen:");
                ui.add(
                    egui::DragValue::new(&mut self.target_distance_mm)
                        .speed(10)
                        .clamp_range(0.0..=100000.),
                );
                ui.label("mm");
            });
        });
    }

    fn plot_csf(&self) -> Line {
        let values = self
            .csf
            .plot_points(0., 50., 4096)
            .map(|(x, y)| Value::new(x, y));
        Line::new(Values::from_values_iter(values))
    }
}
