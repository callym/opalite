use std::sync::Arc;
use hal::{ self, command, Device as _Device };
use gfx::{ self, Device };
use back::{ self, Backend as B };
use crate::renderer::{ ColorFormat, pipeline::pipe };

pub struct Framebuffers {
    dimensions: (u32, u32),
    backbuffers: Arc<Vec<gfx::Backbuffer<B, ColorFormat>>>,
    framebuffers: Vec<gfx::handle::raw::Framebuffer<B>>,
    frame_rtvs: Vec<gfx::memory::Typed<gfx::handle::raw::ImageView<B>, ColorFormat>>,
    scissor: command::Rect,
    viewport: command::Viewport,
}

impl Framebuffers {
    pub fn new(device: &mut Device<B>, (width, height): (u32, u32), pipeline: &pipe::Meta<back::Backend>, backbuffers: Arc<Vec<gfx::Backbuffer<B, ColorFormat>>>) -> Self {
        let image_range = gfx::image::SubresourceRange {
            aspects: hal::format::AspectFlags::COLOR,
            levels: 0..1,
            layers: 0..1,
        };

        let frame_rtvs = backbuffers.iter().map(|backbuffer| {
            device.create_image_view(&backbuffer.color, image_range.clone())
                .unwrap()
        }).collect::<Vec<_>>();

        let framebuffers = frame_rtvs.iter().map(|rtv| {
            let extent = hal::device::Extent { width, height, depth: 1 };
            device.create_framebuffer(pipeline, &[rtv.as_ref()], extent)
                .unwrap()
        }).collect::<Vec<_>>();

        let scissor = command::Rect {
			x: 0, y: 0,
			w: width as u16, h: height as u16,
		};

		let viewport = command::Viewport {
			rect: scissor,
			depth: 0.0 .. 1.0,
		};

        Self {
            dimensions: (width, height),
            backbuffers,
            framebuffers,
            frame_rtvs,
            scissor,
            viewport,
        }
    }

    pub fn scissor(&self) -> command::Rect {
        self.scissor
    }

    pub fn viewport(&self) -> command::Viewport {
        self.viewport.clone()
    }

    pub fn get_frame_resources(&self, id: usize) -> (&gfx::Backbuffer<B, ColorFormat>, &gfx::handle::raw::Framebuffer<B>, &gfx::memory::Typed<gfx::handle::raw::ImageView<B>, ColorFormat>) {
        let backbuffer = &self.backbuffers[id];
		let frame_rtv = &self.frame_rtvs[id];
		let framebuffer = &self.framebuffers[id];

        (backbuffer, framebuffer, frame_rtv)
    }
}
