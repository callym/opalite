use std::{ collections::HashMap, mem, sync::{ Arc, Mutex } };
use failure::Error;
use crate::{ Config, RLock };
use crate::renderer::{ self, Buffer, BufferData, ImageKey, Image, MaterialDesc, Material, ModelKey, Model, RenderError, PushConstant, Sampler, ShaderKey, Shader };
use crate::renderer::pipe::{ PipeKey, Pipe };
use crate::renderer::model::Vertex;

use back;
use back::Backend as B;

use hal;
use hal::{ command, format as f, image as i, memory as m, pass, pso };
use hal::{ Backend, Device };
use hal::{
    DescriptorPool,
    Primitive,
    Backbuffer,
};
use hal::format::Swizzle;
use hal::pass::Subpass;
use hal::pso::{ PipelineStage, ShaderStageFlags };

#[derive(PushConstant, Serialize, Copy, Clone, Debug)]
pub struct ModelLocals {
    pub model: [[f32; 4]; 4],
}

#[derive(BufferData, Serialize, Copy, Clone, Debug)]
#[uniform]
pub struct Locals {
    pub proj_view: [[f32; 4]; 4],
}

pub struct MainPipe {
    _device: Arc<Mutex<back::Device>>,
    viewport: pso::Viewport,
    pipeline_layout: <B as Backend>::PipelineLayout,
    render_pass: <B as Backend>::RenderPass,
    pipeline: <B as Backend>::GraphicsPipeline,
    framebuffers: Vec<<B as Backend>::Framebuffer>,
    images: HashMap<ImageKey, Image<B>>,
    materials: HashMap<MaterialDesc, Material>,
    models: HashMap<ModelKey, RLock<Model>>,
    locals: Buffer<Locals, B>,
    sampler: Arc<Sampler<B>>,
    set_layout: <B as Backend>::DescriptorSetLayout,
}

impl Pipe for MainPipe {
    type Locals = Locals;
    type Models = Model;
    type ModelsLocals = ModelLocals;

    fn key(&self) -> PipeKey {
        PipeKey(String::from("Main"))
    }

    fn locals(&self) -> &Buffer<Self::Locals, B> {
        &self.locals
    }

    fn locals_mut(&mut self) -> &mut Buffer<Self::Locals, B> {
        &mut self.locals
    }
}

impl MainPipe {
    pub fn images(&self) -> &HashMap<ImageKey, Image<B>> {
        &self.images
    }

    pub fn images_mut(&mut self) -> &mut HashMap<ImageKey, Image<B>> {
        &mut self.images
    }

    pub fn materials(&self) -> &HashMap<MaterialDesc, Material> {
        &self.materials
    }

    pub fn materials_mut(&mut self) -> &mut HashMap<MaterialDesc, Material> {
        &mut self.materials
    }

    pub fn models(&self) -> &HashMap<ModelKey, RLock<<Self as Pipe>::Models>> {
        &self.models
    }

    pub fn models_mut(&mut self) -> &mut HashMap<ModelKey, RLock<<Self as Pipe>::Models>> {
        &mut self.models
    }

    pub fn sampler(&self) -> Arc<Sampler<B>> {
        self.sampler.clone()
    }

    pub fn set_layout(&self) -> &<B as Backend>::DescriptorSetLayout {
        &self.set_layout
    }

    pub fn draw(&mut self, command_buffer: &mut command::CommandBuffer<B, hal::Graphics>, frame_id: usize, model_locals: &[(&ModelKey, MaterialDesc, <Self as Pipe>::ModelsLocals)]) {
        let Self {
            images,
            materials,
            pipeline_layout,
            render_pass,
            pipeline,
            framebuffers,
            viewport,
            ..
        } = self;

        command_buffer.set_viewports(&[viewport.clone()]);
        command_buffer.set_scissors(&[viewport.rect]);
        command_buffer.bind_graphics_pipeline(pipeline);

        {
            let mut encoder = command_buffer.begin_render_pass_inline(
                &render_pass,
                &framebuffers[frame_id],
                viewport.rect,
                &[
                    command::ClearValue::Color(command::ClearColor::Float([0.8, 0.8, 0.8, 1.0])),
                    command::ClearValue::DepthStencil(command::ClearDepthStencil(1.0, 0)),
                ],
            );

            for (model_key, material, model_locals) in model_locals {
                let model = self.models.get(model_key).unwrap();
                let model = model.read().unwrap();

                let material = self.materials.get(material).unwrap();
                encoder.bind_graphics_descriptor_sets(pipeline_layout, 0, Some(&material.descriptor_set));

                encoder.push_graphics_constants(
                    pipeline_layout,
                    ShaderStageFlags::VERTEX,
                    0,
                    &model_locals.data()[..],
                );

                encoder.push_graphics_constants(
                    pipeline_layout,
                    ShaderStageFlags::FRAGMENT,
                    <Self as Pipe>::ModelsLocals::SIZE,
                    &material.data()[..],
                );

                encoder.bind_vertex_buffers(pso::VertexBufferSet(vec![(model.vertex_buffer.buffer(), 0)]));
                encoder.bind_index_buffer(hal::buffer::IndexBufferView {
                    buffer: model.index_buffer.buffer(),
                    offset: 0,
                    index_type: hal::IndexType::U32,
                });
                encoder.draw_indexed(0..model.index_buffer.len(), 0, 0..1);
            }
        }
    }

    pub fn new(
        backbuffer: &hal::Backbuffer<B>,
        config: &Config,
        dimensions: (u32, u32),
        dpi_factor: f32,
        device: Arc<Mutex<back::Device>>,
        memory_types: &[hal::MemoryType],
        surface_format: f::Format,
        depth_format: Option<f::Format>,
    ) -> Result<Self, Error> {
        let (width, height) = dimensions;
        let depth_format = depth_format.unwrap();

        let set_layout = {
            let device = device.lock().unwrap();
            let mut bindings = vec![
                pso::DescriptorSetLayoutBinding {
                    binding: 0,
                    ty: pso::DescriptorType::UniformBuffer,
                    count: 1,
                    stage_flags: ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT,
                },
            ];
            bindings.extend(Image::<B>::descriptor_set_binding(
                ShaderStageFlags::FRAGMENT,
                1,
            ));

            device.create_descriptor_set_layout(&bindings[..])
        };

        let pipeline_layout = {
            let device = device.lock().unwrap();
            device.create_pipeline_layout(Some(&set_layout), &[
                (ShaderStageFlags::VERTEX, 0..<Self as Pipe>::ModelsLocals::SIZE),
                (ShaderStageFlags::FRAGMENT, (<Self as Pipe>::ModelsLocals::SIZE)..(<Self as Pipe>::ModelsLocals::SIZE + Material::SIZE)),
            ])
        };

        println!("OFFSET: {}", <Self as Pipe>::ModelsLocals::SIZE);

        let render_pass = {
            let device = device.lock().unwrap();

            let attachment = pass::Attachment {
                format: Some(surface_format),
                ops: pass::AttachmentOps::new(pass::AttachmentLoadOp::Clear, pass::AttachmentStoreOp::Store),
                stencil_ops: pass::AttachmentOps::DONT_CARE,
                layouts: i::ImageLayout::Undefined .. i::ImageLayout::Present,
            };

            let depth_attachment = pass::Attachment {
                format: Some(depth_format),
                ops: pass::AttachmentOps::new(pass::AttachmentLoadOp::Clear, pass::AttachmentStoreOp::DontCare),
                stencil_ops: pass::AttachmentOps::DONT_CARE,
                layouts: i::ImageLayout::Undefined .. i::ImageLayout::DepthStencilAttachmentOptimal,
            };

            let subpass = pass::SubpassDesc {
                colors: &[(0, i::ImageLayout::ColorAttachmentOptimal)],
                depth_stencil: Some(&(1, i::ImageLayout::DepthStencilAttachmentOptimal)),
                inputs: &[],
                preserves: &[],
            };

            let dependency = pass::SubpassDependency {
                passes: pass::SubpassRef::External .. pass::SubpassRef::Pass(0),
                stages: PipelineStage::COLOR_ATTACHMENT_OUTPUT .. PipelineStage::COLOR_ATTACHMENT_OUTPUT,
                accesses: i::Access::empty() .. (i::Access::COLOR_ATTACHMENT_READ | i::Access::COLOR_ATTACHMENT_WRITE),
            };

            device.create_render_pass(&[attachment, depth_attachment], &[subpass], &[dependency])
        };

        let pipeline = {
            let device = device.lock().unwrap();

            let shader = Shader::load_from_config(config, &ShaderKey::new("main"))?;
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

                pipeline_desc.depth_stencil = Some(pso::DepthStencilDesc {
                    depth: pso::DepthTest::On {
                        fun: pso::Comparison::Less,
                        write: true,
                    },
                    depth_bounds: false,
                    .. Default::default()
                });

                pipeline_desc.vertex_buffers.push(pso::VertexBufferDesc {
                    stride: mem::size_of::<Vertex>() as u32,
                    rate: 0,
                });

                pipeline_desc.attributes.extend(Vertex::desc());

                device.create_graphics_pipeline(&pipeline_desc)?
            };

            device.destroy_shader_module(vs_module);
            device.destroy_shader_module(fs_module);

            pipeline
        };

        let depth_view = {
            let device = device.lock().unwrap();
            let depth_image = device.create_image(i::Kind::D2(width, height, 1, 1), 1, depth_format, i::Usage::DEPTH_STENCIL_ATTACHMENT, i::StorageFlags::empty())?;
            let depth_memory_requirements = device.get_image_requirements(&depth_image);
            let memory_type = memory_types.iter().enumerate()
                .position(|(id, mem_type)| {
                    depth_memory_requirements.type_mask & (1 << id) != 0 &&
                    mem_type.properties.contains(m::Properties::DEVICE_LOCAL)
                })
                .unwrap()
                .into();

            let depth_memory = device.allocate_memory(memory_type, depth_memory_requirements.size)?;
            let depth_image = device.bind_image_memory(&depth_memory, 0, depth_image)?;
            device.create_image_view(&depth_image, i::ViewKind::D2, depth_format, f::Swizzle::NO, renderer::DEPTH_RANGE.clone())?
        };

        let framebuffers = match backbuffer {
            Backbuffer::Images(images) => {
                let device = device.lock().unwrap();

                let extent = i::Extent { width, height, depth: 1 };
                let pairs = images.iter()
                    .map(|image| {
                        let rtv = device.create_image_view(&image, i::ViewKind::D2, surface_format, Swizzle::NO, renderer::COLOR_RANGE.clone())?;
                        Ok(rtv)
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                let fbos = pairs.iter()
                    .map(|rtv| device.create_framebuffer(&render_pass, vec![rtv, &depth_view], extent))
                    .collect::<Result<Vec<_>, _>>().map_err(|_| RenderError::FramebufferCreation)?;

                fbos
            },
            Backbuffer::Framebuffer(_) => Err(RenderError::FramebufferCreation)?,
        };

        let locals = Buffer::<<Self as Pipe>::Locals, B>::new(device.clone(), 1, hal::buffer::Usage::UNIFORM, &memory_types).unwrap();

        let viewport = pso::Viewport {
            rect: pso::Rect {
                x: 0,
                y: 0,
                w: width as _,
                h: height as _,
            },
            depth: 0.0 .. 1.0,
        };

        let sampler = {
            let device = device.lock().unwrap();
            device.create_sampler(i::SamplerInfo::new(
                i::Filter::Linear,
                i::WrapMode::Clamp,
            ))
        };

        Ok(Self {
            _device: device,
            viewport,
            pipeline_layout,
            render_pass,
            pipeline,
            framebuffers,
            images: HashMap::new(),
            materials: HashMap::new(),
            models: HashMap::new(),
            locals,
            sampler: Arc::new(Sampler::new(sampler)),
            set_layout,
        })
    }
}
