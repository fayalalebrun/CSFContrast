use cgmath::Vector2;
use egui::plot::{Line, Plot, VLine, Value, Values};
use glium::{backend::Facade, Frame, Surface, Texture2d};

use crate::{csf::Csf, grating::Grating, perception_adapter::PerceptionAdapter};

pub struct System {
    grating: Grating,
    intermediate: Option<Texture2d>,
    screen_dims_mm: Vector2<f32>,
    screen_distance_mm: f32,
    csf: Csf,
    adapter: PerceptionAdapter,
    adapt: bool,
}

impl System {
    pub fn new(facade: &dyn Facade) -> Self {
        let grating = Grating::new(facade);
        Self {
            grating,
            intermediate: None,
            screen_dims_mm: (600.0f32, 336.0f32).into(),
            screen_distance_mm: 750.,
            csf: Csf {
                a: 1.787,
                ω: 7.22,
                σ: 2.2,
                k: 0.71,
            },
            adapter: PerceptionAdapter::new(facade),
            adapt: true,
        }
    }

    pub fn draw(&mut self, facade: &dyn Facade, surface: &mut Frame) {
        if self.intermediate.is_none()
            || self.intermediate.as_ref().unwrap().dimensions() != surface.get_dimensions()
        {
            self.intermediate = Some(
                Texture2d::empty_with_format(
                    facade,
                    glium::texture::UncompressedFloatFormat::U16U16U16U16,
                    glium::texture::MipmapsOption::NoMipmap,
                    surface.get_dimensions().0,
                    surface.get_dimensions().1,
                )
                .unwrap(),
            );
        }

        let mut int_surface = self.intermediate.as_ref().unwrap().as_surface();
        self.grating.draw(&mut int_surface);
        if self.adapt {
            self.adapter.draw(
                self.intermediate.as_ref().unwrap(),
                surface,
                &self.csf,
                self.cycles_per_degree(),
            );
        } else {
            int_surface.fill(surface, glium::uniforms::MagnifySamplerFilter::Nearest);
        }
    }

    fn total_visual_angle(&self) -> f32 {
        2. * (self.screen_dims_mm.x / self.screen_distance_mm).atan()
    }

    fn cycles_per_degree(&self) -> f32 {
        self.grating.frequency() / self.total_visual_angle().to_degrees()
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
                self.total_visual_angle().to_degrees()
            ));

            ui.label(format!(
                "Spatial density: {} c/deg",
                self.cycles_per_degree()
            ));

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
                    plot_ui.vline(VLine::new(self.cycles_per_degree()).name("current"));
                    plot_ui.vline(VLine::new(self.adapter.target_cpd).name("target"));
                });
            ui.heading("Adjustment Algorithm");
            ui.checkbox(&mut self.adapt, "Activate");
            ui.horizontal(|ui| {
                ui.label("Target:");
                ui.add(
                    egui::DragValue::new(&mut self.adapter.target_cpd)
                        .speed(0.1)
                        .clamp_range(1.0..=50.),
                );
                ui.label("c/deg");
            });
        });
    }

    fn plot_csf(&self) -> Line {
        let max_cdg = 50.;
        let points_n = 1000;
        let csf = (1..=points_n).map(|i| {
            let x = (i as f32 / points_n as f32) * max_cdg;
            Value::new(x, self.csf.apply(x))
        });
        Line::new(Values::from_values_iter(csf))
    }
}
