use glium::{glutin, Display};
use gstreamer::prelude::*;

use gstreamer_gl::prelude::*;
use gstreamer_gl::GLContext;
use gstreamer_gl::GLDisplay;

#[derive(Clone)]
pub struct CtxInfo {
    pub(crate) gl_context: GLContext,
    pub(crate) gl_display: GLDisplay,
}

pub enum SurfaceType<'a> {
    Display(&'a Display),
    Headless(&'a glium::backend::glutin::headless::Headless),
}

impl CtxInfo {
    pub fn new(surface: SurfaceType) -> Self {
        gstreamer::init().unwrap();

        #[cfg(target_os = "linux")]
        let (gl_context, gl_display, platform, api) = {
            let (wayland_display, xlib_display) = {
                match surface {
                    SurfaceType::Display(display) => {
                        let gl_window = display.gl_window();
                        let inner_window = gl_window.window();
                        (inner_window.wayland_display(), inner_window.xlib_display())
                    }
                    SurfaceType::Headless(_) => (None, None),
                }
            };

            use glutin::platform::unix::RawHandle;
            use glutin::platform::unix::WindowExtUnix;
            use glutin::platform::ContextTraitExt;

            let (raw_handle, api, egl_display) = unsafe {
                match surface {
                    SurfaceType::Display(display) => (
                        display.gl_window().raw_handle(),
                        display.gl_window().get_api(),
                        display.gl_window().get_egl_display(),
                    ),
                    SurfaceType::Headless(headless) => (
                        headless.gl_context().raw_handle(),
                        headless.gl_context().get_api(),
                        headless.gl_context().get_egl_display(),
                    ),
                }
            };

            let api = match api {
                glutin::Api::OpenGl => gstreamer_gl::GLAPI::OPENGL3,
                glutin::Api::OpenGlEs => gstreamer_gl::GLAPI::GLES2,
                _ => gstreamer_gl::GLAPI::empty(),
            };

            match raw_handle {
                RawHandle::Egl(egl_context) => {
                    let mut gl_display = None;

                    if let Some(display) = egl_display {
                        gl_display = Some(
                            unsafe {
                                gstreamer_gl_egl::GLDisplayEGL::with_egl_display(display as usize)
                            }
                            .unwrap()
                            .upcast::<gstreamer_gl::GLDisplay>(),
                        )
                    };

                    if let Some(display) = wayland_display {
                        gl_display = Some(
                            unsafe {
                                gstreamer_gl_wayland::GLDisplayWayland::with_display(
                                    display as usize,
                                )
                            }
                            .unwrap()
                            .upcast::<gstreamer_gl::GLDisplay>(),
                        )
                    };

                    (
                        egl_context as usize,
                        gl_display.expect("Could not retrieve GLDisplay through EGL context and/or Wayland display"),
                        gstreamer_gl::GLPlatform::EGL,
			api
                    )
                }
                RawHandle::Glx(glx_context) => {
                    let gl_display = if let Some(display) = xlib_display {
                        unsafe { gstreamer_gl_x11::GLDisplayX11::with_display(display as usize) }
                            .unwrap()
                    } else {
                        panic!("X11 window without X Display");
                    };

                    (
                        glx_context as usize,
                        gl_display.upcast::<gstreamer_gl::GLDisplay>(),
                        gstreamer_gl::GLPlatform::GLX,
                        api,
                    )
                }
            }
        };

        #[cfg(target_os = "macos")]
        let (gl_context, gl_display, platform, api) = {
            match surface {
                SurfaceType::Display(&display) => {
                    let context = unsafe { display.gl_window().raw_handle() as usize };
                    let platform = gstreamer_gl::GLPlatform::CGL;
                    let api = match display.gl_window().get_api() {
                        glutin::Api::OpenGl => gstreamer_gl::GLAPI::OPENGL3,
                        glutin::Api::OpenGlEs => gstreamer_gl::GLAPI::GLES2,
                        _ => gstreamer_gl::GLAPI::empty(),
                    };
                    (context, gstreamer_gl::GLDisplay::new(), platform, api)
                }
                SurfaceType::Headless(_) => panic!("Not supported"),
            }
        };

        // Setup a shared_context
        let shared_context =
            unsafe { gstreamer_gl::GLContext::new_wrapped(&gl_display, gl_context, platform, api) }
                .unwrap();
        shared_context
            .activate(true)
            .expect("Couldn't activate wrapped GL context");
        shared_context.fill_info().unwrap();

        Self {
            gl_context: shared_context,
            gl_display,
        }
    }
}
