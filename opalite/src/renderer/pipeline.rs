use std::{ ops::Drop, sync::{ Arc, Mutex } };
use hal::{ self, pso, Primitive, Device as _Device };
use gfx::{ self, Device };
use back::Backend as B;
use winit::Window;
use crate::renderer::{ ColorFormat, Framebuffers, Shader, Vertex };

gfx_descriptors! {
    desc {
        sampled_image: pso::SampledImage,
        sampler: pso::Sampler,
    }
}

gfx_graphics_pipeline! {
    pipe {
        desc: desc::Component,
        color: gfx::pso::RenderTarget<ColorFormat>,
        vertices: gfx::pso::VertexBuffer<Vertex>,
    }
}

pub struct Pipeline {
    descriptors: (Arc<desc::Set<B>>, Arc<desc::Data<B>>),
    device: Arc<Mutex<Device<B>>>,
    framebuffers: Framebuffers,
    pipeline: pipe::Meta<B>,
    shader_modules: Option<(<B as hal::Backend>::ShaderModule, <B as hal::Backend>::ShaderModule)>,
}

impl Pipeline {
    pub fn new(
                window: &Window,
                device_arc: Arc<Mutex<Device<B>>>,
                desc: Arc<desc::Set<B>>,
                desc_data: Arc<desc::Data<B>>,
                shader: Shader,
                backbuffers: Arc<Vec<gfx::Backbuffer<B, ColorFormat>>>,
    ) -> Self {
        let mut device = device_arc.lock().unwrap();
        let vs = device.raw.create_shader_module(&shader.vertex).unwrap();
        let fs = device.raw.create_shader_module(&shader.fragment).unwrap();

        let pipe_init = pipe::Init {
            desc: &desc,
            color: pso::ColorBlendDesc(pso::ColorMask::ALL, pso::BlendState::ALPHA),
            vertices: (),
        };

        let pipeline = device.create_graphics_pipeline(
            pso::GraphicsShaderSet {
                vertex: pso::EntryPoint { entry: "main", module: &vs, specialization: &[] },
                hull: None,
                domain: None,
                geometry: None,
                fragment: Some(pso::EntryPoint { entry: "main", module: &fs, specialization: &[] }),
            },
            Primitive::TriangleList,
            pso::Rasterizer::FILL,
            pipe_init,
        ).unwrap();

        let dimensions = window.get_inner_size().unwrap();

        let framebuffers = Framebuffers::new(&mut *device, dimensions, &pipeline, backbuffers);

        Self {
            device: device_arc.clone(),
            descriptors: (desc, desc_data),
            framebuffers,
            pipeline,
            shader_modules: Some((vs, fs)),
        }
    }

    pub fn descriptors(&self) -> (&desc::Set<B>, &desc::Data<B>) {
        (&self.descriptors.0, &self.descriptors.1)
    }

    pub fn framebuffers(&self) -> &Framebuffers {
        &self.framebuffers
    }

    pub fn pipeline(&self) -> &pipe::Meta<B> {
        &self.pipeline
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        match self.shader_modules.take() {
            Some((vs, fs)) => {
                let device = self.device.lock().unwrap();
                device.raw.destroy_shader_module(vs);
                device.raw.destroy_shader_module(fs);
            },
            None => (),
        };
    }
}
