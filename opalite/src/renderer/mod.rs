use std::{ collections::HashMap, mem, ops::Drop, sync::{ Arc, Mutex } };
use failure::{ self, Error };
use specs::{ Fetch, ReadStorage, System };
use winit::Window;
use crate::{ Config, Position, WindowClosed };

use back;
use back::Backend as B;

use hal;
use hal::{ command, device as d, format as f, image as i, memory as m, pass, pso, pool };
use hal::{ Device, Instance, PhysicalDevice, Surface, Swapchain };
use hal::{
    Adapter,
    DescriptorPool,
    FrameSync,
    Primitive,
    Backbuffer,
    SwapchainConfig,
};
use hal::format::{ AsFormat, ChannelType, Rgba8Srgb as ColorFormat, Swizzle };
use hal::pass::Subpass;
use hal::pso::{ PipelineStage, ShaderStageFlags, Specialization };
use hal::queue::Submission;

mod buffer;
pub use self::buffer::{ Buffer, BufferData };

mod model;
pub use self::model::{ ModelKey, Model, ModelType };

mod shader;
pub use self::shader::{ ShaderKey, Shader };

const COLOR_RANGE: i::SubresourceRange = i::SubresourceRange {
    aspects: f::Aspects::COLOR,
    levels: 0 .. 1,
    layers: 0 .. 1,
};

#[derive(BufferData, Copy, Clone, Debug)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
    // pads to 8
    _padding: [f32; 2],
}

impl Vertex {
    pub fn new(position: [f32; 3], color: [f32; 3]) -> Self {
        Self { position, color, _padding: [0.0, 0.0] }
    }
}

#[derive(BufferData, Copy, Clone, Debug)]
pub struct Locals {

}

#[derive(Fail, Debug)]
pub enum RenderError {
    #[fail(display = "Cannot get window size.")]
    WindowSize,
    #[fail(display = "No vaild adapters found.")]
    ChooseAdapters,
    #[fail(display = "No valid surface format found.")]
    NoSurfaceFormat,
    #[fail(display = "{} Shader Module creating failed.", _0)]
    ShaderModuleFail(&'static str),
    #[fail(display = "Framebuffer error.")]
    FramebufferCreation,
}

pub struct Renderer {
    models: HashMap<ModelKey, Model>,
}

fn choose_adapters(mut adapters: Vec<Adapter<B>>) -> Result<Adapter<B>, Error> {
    if adapters.len() == 0 {
        Err(RenderError::ChooseAdapters)?;
    }

    // choose best adapter here
    Ok(adapters.remove(0))
}

impl Renderer {
    pub fn new(config: Config, window: &Window) -> Result<Self, Error> {
        let (width, height) = window.get_inner_size().ok_or(RenderError::WindowSize)?;

        let instance = back::Instance::create(&config.title, 1);
        let mut surface = instance.create_surface(window);
        let adapter = {
            let adapters = instance.enumerate_adapters();
            for adapter in &adapters {
                println!("{:?}", adapter.info);
            }
            choose_adapters(adapters)?
        };
        let surface_format = surface.capabilities_and_formats(&adapter.physical_device).1
            .map_or(Some(f::Format::Rgba8Srgb), |f| f.into_iter().find(|f| f.base_format().1 == ChannelType::Srgb))
            .ok_or(RenderError::NoSurfaceFormat)?;
        let memory_types = adapter.physical_device.memory_properties().memory_types;
        let limits = adapter.physical_device.limits();

        let (device, mut queue_group) = adapter.open_with::<_, hal::Graphics>(1, |f| surface.supports_queue_family(f))?;
        let device = Arc::new(Mutex::new(device));

        let mut command_pool = {
            let device = device.lock().unwrap();
            device.create_command_pool_typed(&queue_group, pool::CommandPoolCreateFlags::empty(), 16)
        };

        let mut queue = &mut queue_group.queues[0];

        println!("Surface format: {:?}", surface_format);
        let swap_config = SwapchainConfig::new()
            .with_color(surface_format);
        let (mut swap_chain, backbuffer) = {
            let device = device.lock().unwrap();
            device.create_swapchain(&mut surface, swap_config)
        };

        // TODO - move layouts to config files!
        let set_layout = {
            let device = device.lock().unwrap();
            device.create_descriptor_set_layout(&[
                pso::DescriptorSetLayoutBinding {
                    binding: 0,
                    ty: pso::DescriptorType::UniformBuffer,
                    count: 1,
                    stage_flags: ShaderStageFlags::FRAGMENT,
                }
            ])
        };

        let pipeline_layout = {
            let device = device.lock().unwrap();
            device.create_pipeline_layout(Some(&set_layout), &[])
        };

        let render_pass = {
            let device = device.lock().unwrap();

            let attachment = pass::Attachment {
                format: Some(surface_format),
                ops: pass::AttachmentOps::new(pass::AttachmentLoadOp::Clear, pass::AttachmentStoreOp::Store),
                stencil_ops: pass::AttachmentOps::DONT_CARE,
                layouts: i::ImageLayout::Undefined .. i::ImageLayout::Present,
            };

            let subpass = pass::SubpassDesc {
                colors: &[(0, i::ImageLayout::ColorAttachmentOptimal)],
                depth_stencil: None,
                inputs: &[],
                preserves: &[],
            };

            let dependency = pass::SubpassDependency {
                passes: pass::SubpassRef::External .. pass::SubpassRef::Pass(0),
                stages: PipelineStage::COLOR_ATTACHMENT_OUTPUT .. PipelineStage::COLOR_ATTACHMENT_OUTPUT,
                accesses: i::Access::empty() .. (i::Access::COLOR_ATTACHMENT_READ | i::Access::COLOR_ATTACHMENT_WRITE),
            };

            device.create_render_pass(&[attachment], &[subpass], &[dependency])
        };

        let pipeline = {
            let device = device.lock().unwrap();

            let shader = Shader::load_from_config(&config, &ShaderKey::new("main"))?;
            let vs_module = device.create_shader_module(&shader.vertex[..])
                .map_err(|_| RenderError::ShaderModuleFail("Vertex"))?;
            let fs_module = device.create_shader_module(&shader.fragment[..])
                .map_err(|_| RenderError::ShaderModuleFail("Fragment"))?;

            let pipeline = {
                let (vs_entry, fs_entry) = (
                    pso::EntryPoint::<B> {
                        entry: "main",
                        module: &vs_module,
                        specialization: &[],
                    },
                    pso::EntryPoint::<B> {
                        entry: "main",
                        module: &fs_module,
                        specialization: &[],
                    },
                );

                let shader_entries = pso::GraphicsShaderSet {
                    vertex: vs_entry,
                    hull: None,
                    domain: None,
                    geometry: None,
                    fragment: Some(fs_entry),
                };

                let subpass = Subpass { index: 0, main_pass: &render_pass };

                let mut pipeline_desc = pso::GraphicsPipelineDesc::new(
                    shader_entries,
                    Primitive::TriangleList,
                    pso::Rasterizer::FILL,
                    &pipeline_layout,
                    subpass,
                );
                pipeline_desc.blender.targets.push(pso::ColorBlendDesc(pso::ColorMask::ALL, pso::BlendState::ALPHA));

                pipeline_desc.vertex_buffers.push(pso::VertexBufferDesc {
                    stride: mem::size_of::<Vertex>() as u32,
                    rate: 0,
                });
                // TODO - find a way to automatically impl these
                // Vertex.position
                pipeline_desc.attributes.push(pso::AttributeDesc {
                    location: 0,
                    binding: 0,
                    element: pso::Element {
                        // vec3
                        format: f::Format::Rgb32Float,
                        offset: 0,
                    },
                });
                // Vertex.color
                pipeline_desc.attributes.push(pso::AttributeDesc {
                    location: 0,
                    binding: 0,
                    element: pso::Element {
                        // vec3
                        format: f::Format::Rgb32Float,
                        // size of previous element - (position, vec3) - in bytes
                        offset: 12,
                    },
                });

                device.create_graphics_pipeline(&pipeline_desc)
            };

            device.destroy_shader_module(vs_module);
            device.destroy_shader_module(fs_module);

            pipeline
        };

        let mut desc_pool = {
            let device = device.lock().unwrap();

            device.create_descriptor_pool(
                1,
                &[
                    pso::DescriptorRangeDesc {
                        ty: pso::DescriptorType::UniformBuffer,
                        count: 1,
                    }
                ],
            )
        };

        let (frame_images, framebuffers) = match backbuffer {
            Backbuffer::Images(images) => {
                let device = device.lock().unwrap();

                let extent = d::Extent { width, height, depth: 1 };
                let pairs = images.into_iter()
                    .map(|image| {
                        let rtv = device.create_image_view(&image, surface_format, Swizzle::NO, COLOR_RANGE.clone());
                        match rtv {
                            Ok(rtv) => Ok((image, rtv)),
                            Err(err) => Err(err),
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let fbos = pairs.iter()
                    .map(|&(_, ref rtv)| device.create_framebuffer(&render_pass, Some(rtv), extent))
                    .collect::<Result<Vec<_>, _>>().map_err(|_| RenderError::FramebufferCreation)?;

                (pairs, fbos)
            },
            Backbuffer::Framebuffer(fbo) => (Vec::new(), vec![fbo]),
        };

        println!("Memory types: {:?}", memory_types);

        let vertex_buffer = Buffer::<Vertex, B>::new(device.clone(), 1, hal::buffer::Usage::VERTEX, &memory_types)?;

        let mut local_buffer = Buffer::<Locals, B>::new(device.clone(), 1, hal::buffer::Usage::UNIFORM, &memory_types)?;
        local_buffer.write(Locals {

        });

        let viewport = command::Viewport {
            rect: command::Rect {
                x: 0,
                y: 0,
                w: width as _,
                h: height as _,
            },
            depth: 0.0 .. 1.0,
        };

        let (mut frame_semaphore, mut frame_fence) = {
            let device = device.lock().unwrap();
            // TODO: remove fence
            (device.create_semaphore(), device.create_fence(false))
        };

        Ok(Self {
            models: HashMap::new(),
        })
    }

    pub fn load_model(&mut self, key: &ModelKey) -> &Model {
        let model = match key.ty() {
            ModelType::File(_) => unimplemented!(),
            ModelType::Quad => Model { },
        };

        self.models.insert(key.clone(), model);
        self.models.get(&key).unwrap()
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {

    }
}

impl<'a> System<'a> for Renderer {
    type SystemData = (ReadStorage<'a, Position>, ReadStorage<'a, ModelKey>, Fetch<'a, WindowClosed>);

    fn run(&mut self, (positions, model_keys, window_closed): Self::SystemData) {
        if *window_closed == true {
            println!("Window Closed");
            return;
        }

        use specs::Join;

        for model_key in model_keys.join() {
            match self.models.get(model_key) {
                None => { self.load_model(model_key); },
                _ => ()
            };
        };

        for (position, model_key) in (&positions, &model_keys).join() {
            // this unwrap is safe because all the models are added at the top of the function.
            let model = self.models.get(model_key).unwrap();
        }
    }
}
