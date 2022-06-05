use std::cell::Cell;

use egui_glium::EguiGlium;
use glium::{
    backend::Facade,
    glutin::{self, event_loop::ControlFlow},
    Display, Frame,
};

pub fn run(
    mut gui_paint: Box<dyn FnMut(&egui::Context)>,
    mut draw_clos: Box<dyn FnMut(&mut Frame, &dyn Facade)>,
    display: Display,
    event_loop: glutin::event_loop::EventLoop<()>,
) {
    let draw_gui = Cell::new(true);
    let mut egui_glium = egui_glium::EguiGlium::new(&display);
    let mut redraw = move |display: &Display,
                           egui_glium: &mut EguiGlium,
                           _control_flow: &mut ControlFlow,
                           draw_gui: bool| {
        egui_glium.run(&display, &mut gui_paint);

        {
            let mut target = display.draw();
            draw_clos(&mut target, display);
            if draw_gui {
                egui_glium.paint(&display, &mut target);
            }
            target.finish().unwrap();
        }
    };

    event_loop.run(move |event, _, control_flow| {
        match event {
            // Platform-dependent event handlers to workaround a winit bug
            // See: https://github.com/rust-windowing/winit/issues/987
            // See: https://github.com/rust-windowing/winit/issues/1619
            glutin::event::Event::RedrawEventsCleared if cfg!(windows) => {
                redraw(&display, &mut egui_glium, control_flow, draw_gui.get())
            }
            glutin::event::Event::RedrawRequested(_) if !cfg!(windows) => {
                redraw(&display, &mut egui_glium, control_flow, draw_gui.get())
            }
            glutin::event::Event::MainEventsCleared => {
                display.gl_window().window().request_redraw();
            }

            glutin::event::Event::WindowEvent { event, .. } => {
                use glutin::event::WindowEvent;
                match event {
                    WindowEvent::KeyboardInput {
                        input:
                            glutin::event::KeyboardInput {
                                virtual_keycode: Some(glutin::event::VirtualKeyCode::G),
                                state: glutin::event::ElementState::Pressed,
                                ..
                            },
                        ..
                    } => {
                        draw_gui.set(!draw_gui.get());
                    }
                    _ => (),
                }
                if matches!(event, WindowEvent::CloseRequested | WindowEvent::Destroyed) {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                }

                egui_glium.on_event(&event);

                display.gl_window().window().request_redraw(); // TODO: ask egui if the events warrants a repaint instead
            }

            _ => (),
        }
    });
}
