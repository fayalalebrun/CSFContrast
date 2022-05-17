use std::{cell::RefCell, rc::Rc};

use glium::glutin;
use system::System;

mod csf;
mod fft;
mod grating;
mod gui;
mod image_shader;
mod perception_adapter;
mod system;

fn main() {
    let event_loop = glutin::event_loop::EventLoop::with_user_event();
    let display = create_display(&event_loop);

    let system = Rc::new(RefCell::new(System::new(&display)));

    gui::run(
        Box::new({
            let system = system.clone();
            move |egui_ctx| {
                system.borrow_mut().draw_ui(egui_ctx);
            }
        }),
        Box::new(move |target, facade| {
            use glium::Surface as _;
            target.clear_color(0.1, 0.2, 0.3, 1.0);
            system.borrow_mut().draw(facade, target);
        }),
        display,
        event_loop,
    );
}

fn create_display(event_loop: &glutin::event_loop::EventLoop<()>) -> glium::Display {
    let window_builder = glutin::window::WindowBuilder::new()
        .with_resizable(true)
        .with_fullscreen(Some(glutin::window::Fullscreen::Borderless(None)));

    let context_builder = glutin::ContextBuilder::new()
        .with_srgb(true)
        .with_vsync(true)
        .with_gl_profile(glutin::GlProfile::Core)
        .with_gl(glutin::GlRequest::Latest);

    glium::Display::new(window_builder, context_builder, event_loop).unwrap()
}
