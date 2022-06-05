use std::time::Duration;

use glium::{
    backend::Facade,
    texture::{Dimensions, MipmapsOption, UncompressedFloatFormat},
};
use gstreamer::prelude::*;
use gstreamer_gl::{prelude::*, GLContext, GLDisplay};

mod context;

pub use context::{CtxInfo, SurfaceType};

use crate::image_shader::ImageShader;

pub struct Gstreamer {
    pipeline: gstreamer::Pipeline,
    pub appsink: gstreamer_app::AppSink,
    gl_context: GLContext,
    texture: Option<glium::Texture2d>,
    copy_shader: ImageShader,
}

impl Gstreamer {
    pub fn new(facade: &dyn Facade, ctx_info: &CtxInfo) -> Self {
        let (pipeline, appsink) =
            Self::create_pipeline(ctx_info.gl_context.clone(), ctx_info.gl_display.clone());

        let bin = pipeline.upcast::<gstreamer::Bin>();

        let pipeline = bin.downcast::<gstreamer::Pipeline>().unwrap();

        let copy_shader = ImageShader::new(facade, include_str!("./gstreamer/copy_frag.glsl"));

        let input = Self {
            pipeline,
            appsink,
            gl_context: ctx_info.gl_context.clone(),
            texture: None,
            copy_shader,
        };

        input
    }

    fn create_pipeline(
        gl_context: GLContext,
        gl_display: GLDisplay,
    ) -> (gstreamer::Pipeline, gstreamer_app::AppSink) {
        let (bin, appsink) = Self::create_sink();

        let playbin = gstreamer::ElementFactory::make("playbin", None).unwrap();
        playbin.set_property_from_str("flags", "video");

        playbin.set_property("video-sink", &bin);
        playbin.set_property_from_str("flags", "video");

        let playbin = playbin.downcast::<gstreamer::Pipeline>().unwrap();

        playbin.bus().unwrap().set_sync_handler(move |_, msg| {
            if let gstreamer::MessageView::NeedContext(ctxt) = msg.view() {
                let context_type = ctxt.context_type();
                if context_type == *gstreamer_gl::GL_DISPLAY_CONTEXT_TYPE {
                    if let Some(el) = msg
                        .src()
                        .map(|s| s.downcast::<gstreamer::Element>().unwrap())
                    {
                        let context = gstreamer::Context::new(context_type, true);
                        context.set_gl_display(&gl_display);
                        el.set_context(&context);
                    }
                }
                if context_type == "gst.gl.app_context" {
                    if let Some(el) = msg
                        .src()
                        .map(|s| s.downcast::<gstreamer::Element>().unwrap())
                    {
                        let mut context = gstreamer::Context::new(context_type, true);
                        {
                            let context = context.get_mut().unwrap();
                            let s = context.structure_mut();
                            s.set("context", &gl_context);
                        }
                        el.set_context(&context);
                    }
                }
            };

            gstreamer::BusSyncReply::Pass
        });

        playbin
            .bus()
            .unwrap()
            .add_watch(move |_, msg| {
                use gstreamer::MessageView;

                if let MessageView::Error(err) = msg.view() {
                    eprintln!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    );
                };

                // Tell the mainloop to continue executing this callback.

                Continue(true)
            })
            .expect("Failed to add bus watch");

        (playbin, appsink)
    }

    pub fn create_sink() -> (gstreamer::Element, gstreamer_app::AppSink) {
        let bin = gstreamer::Bin::new(Some("bin"));

        let gl_upload = gstreamer::ElementFactory::make("glupload", None).unwrap();

        let appsink = gstreamer::ElementFactory::make("appsink", Some("app-sink"))
            .unwrap()
            .dynamic_cast::<gstreamer_app::AppSink>()
            .unwrap();

        let caps = gstreamer::Caps::builder("video/x-raw")
            .features(&[&gstreamer_gl::CAPS_FEATURE_MEMORY_GL_MEMORY])
            //.features(&[&gstreamer_video::CAPS_FEATURE_META_GST_VIDEO_GL_TEXTURE_UPLOAD_META])
            .field("format", &gstreamer_video::VideoFormat::Rgba.to_str())
            .field("texture-target", &"2D")
            //.field("width", &1920)
            //.field("height", &1080)
            //.field("framerate", &"25/1")
            .build();
        appsink.set_caps(Some(&caps));

        appsink.set_max_buffers(1);
        appsink.set_drop(true);
        appsink.set_enable_last_sample(true);
        appsink.set_property("emit-signals", &false);

        let gl_convert = gstreamer::ElementFactory::make("glcolorconvert", None).unwrap();
        let gl_flip = gstreamer::ElementFactory::make("glvideoflip", None).unwrap();

        gl_flip.set_property_from_str("video-direction", "vert");

        {
            let elements = [
                &gl_upload,
                &gl_convert,
                &gl_flip,
                appsink.upcast_ref::<gstreamer::Element>(),
            ];

            bin.add_many(&elements).unwrap();

            gstreamer::Element::link_many(&elements).unwrap();
        }
        let pad = bin
            .find_unlinked_pad(gstreamer::PadDirection::Sink)
            .unwrap();
        let ghost_pad = gstreamer::GhostPad::with_target(Some("sink"), &pad).unwrap();
        bin.add_pad(&ghost_pad).unwrap();

        (bin.upcast::<gstreamer::Element>(), appsink)
    }

    pub fn set_uri(&self, uri: &str) {
        self.pipeline.set_state(gstreamer::State::Null).unwrap();

        println!("Next video: {}", uri);
        self.pipeline.set_property("uri", uri);

        self.pipeline.set_state(gstreamer::State::Playing).unwrap();
    }

    pub fn draw(&mut self, facade: &dyn Facade) -> &'_ glium::Texture2d {
        let sample = self.appsink.pull_sample();

        match sample {
            Err(_) => (),
            Ok(sample) => {
                let buffer = sample.buffer_owned().unwrap();
                if let Some(sync) = buffer.meta::<gstreamer_gl::GLSyncMeta>() {
                    sync.wait(&self.gl_context);
                }

                let info = sample
                    .caps()
                    .and_then(|caps| gstreamer_video::VideoInfo::from_caps(caps).ok())
                    .unwrap();

                if let Ok(frame) =
                    gstreamer_video::VideoFrame::from_buffer_readable_gl(buffer, &info)
                {
                    let texture_id = frame.texture_id(0);
                    let width = frame.width();
                    let height = frame.height();
                    if let Some(id) = texture_id {
                        let new_texture = unsafe {
                            glium::texture::Texture2d::from_id(
                                facade,
                                UncompressedFloatFormat::U8U8U8U8,
                                id,
                                false,
                                MipmapsOption::NoMipmap,
                                Dimensions::Texture2d { width, height },
                            )
                        };

                        if self.texture.is_none()
                            || self.texture.as_mut().unwrap().dimensions() != (width, height)
                        {
                            self.texture.replace(
                                glium::texture::Texture2d::empty_with_format(
                                    facade,
                                    UncompressedFloatFormat::U16U16U16U16,
                                    MipmapsOption::NoMipmap,
                                    width,
                                    height,
                                )
                                .unwrap(),
                            );
                        }
                        let mut surface = self.texture.as_ref().unwrap().as_surface();
                        self.copy_shader.draw(
                            &mut surface,
                            &glium::uniform! {
                            tex: new_texture,
                                        },
                        );
                    }
                }
            }
        }

        self.texture.as_ref().unwrap()
    }
}
