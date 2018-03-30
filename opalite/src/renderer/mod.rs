use std::{ collections::HashMap, mem, ops::Drop, sync::{ Arc, Mutex } };
use cgmath::Vector3;
use conrod::{ self, render::{ self, PrimitiveWalker } };
use failure::Error;
use specs::{ Entities, Fetch, FetchMut, ReadStorage, System, WriteStorage };
use winit::Window;
use crate::{ Config, Map, OpalUi, RLock, WindowClosed };

use back;
use back::Backend as B;

use hal;
use hal::{ command, device as d, format as f, image as i, memory as m, pass, pso, pool };
use hal::{ Backend, Device, Instance, PhysicalDevice, Surface, Swapchain };
use hal::{
    Adapter,
    DescriptorPool,
    FrameSync,
    Primitive,
    Backbuffer,
    SwapchainConfig,
};
use hal::format::{ ChannelType, Swizzle };
use hal::pass::Subpass;
use hal::pso::{ PipelineStage, ShaderStageFlags };
use hal::queue::Submission;

mod buffer;
pub use self::buffer::{ Buffer, BufferData };

mod camera;
pub use self::camera::Camera;

pub mod model;
pub use self::model::{ ModelKey, Model, ModelData, ModelType, ProceduralModel, Vertex, UiVertex };

mod push_constant;
pub use self::push_constant::PushConstant;

mod shader;
pub use self::shader::{ ShaderKey, Shader };

const COLOR_RANGE: i::SubresourceRange = i::SubresourceRange {
    aspects: f::Aspects::DEPTH,
    levels: 0 .. 1,
    layers: 0 .. 1,
};

const DEPTH_RANGE: i::SubresourceRange = i::SubresourceRange {
    aspects: f::Aspects::COLOR,
    levels: 0 .. 1,
    layers: 0 .. 1,
};

#[derive(PushConstant, Serialize, Copy, Clone, Debug)]
pub struct ModelLocals {
    model: [[f32; 4]; 4],
}

#[derive(BufferData, Serialize, Copy, Clone, Debug)]
#[uniform]
pub struct Locals {
    proj_view: [[f32; 4]; 4],
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
    command_pool: hal::CommandPool<B, hal::Graphics>,
    device: Arc<Mutex<back::Device>>,
    dimensions: (u32, u32),
    dpi_factor: f32,
    frame_fence: <B as Backend>::Fence,
    frame_semaphore: <B as Backend>::Semaphore,
    _limits: hal::Limits,
    memory_types: Vec<hal::MemoryType>,
    queue_group: hal::QueueGroup<B, hal::Graphics>,
    viewport: hal::command::Viewport,
    swap_chain: <B as Backend>::Swapchain,
    // main shader
    pipeline_layout: <B as Backend>::PipelineLayout,
    render_pass: <B as Backend>::RenderPass,
    pipeline: <B as Backend>::GraphicsPipeline,
    desc_set: <B as Backend>::DescriptorSet,
    framebuffers: Vec<<B as Backend>::Framebuffer>,
    // ui shader
    ui_pipeline_layout: <B as Backend>::PipelineLayout,
    ui_render_pass: <B as Backend>::RenderPass,
    ui_pipeline: <B as Backend>::GraphicsPipeline,
    ui_desc_set: <B as Backend>::DescriptorSet,
    ui_framebuffers: Vec<<B as Backend>::Framebuffer>,
    //
    models: HashMap<ModelKey, RLock<Model>>,
    locals: Buffer<Locals, B>,
    ui: Vec<Model<UiVertex>>,
    //
    _instance: back::Instance,
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
        let dpi_factor = window.hidpi_factor();

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
        let depth_format = f::Format::D32Float;

        let memory_types = adapter.physical_device.memory_properties().memory_types;
        let limits = adapter.physical_device.limits();

        let (device, queue_group) = adapter.open_with::<_, hal::Graphics>(1, |f| surface.supports_queue_family(f))?;
        let device = Arc::new(Mutex::new(device));

        let command_pool = {
            let device = device.lock().unwrap();
            device.create_command_pool_typed(&queue_group, pool::CommandPoolCreateFlags::empty(), 16)
        };

        let swap_config = SwapchainConfig::new()
            .with_color(surface_format)
            .with_depth_stencil(depth_format);

        let (swap_chain, backbuffer) = {
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
                    stage_flags: ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT,
                }
            ])
        };

        let pipeline_layout = {
            let device = device.lock().unwrap();
            device.create_pipeline_layout(Some(&set_layout), &[
                (ShaderStageFlags::VERTEX, 0..ModelLocals::SIZE),
            ])
        };

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

        let desc_set = desc_pool.allocate_set(&set_layout);

        let depth_view = {
            let device = device.lock().unwrap();
            let depth_image = device.create_image(i::Kind::D2(width as u16, height as u16, i::AaMode::Single), 1, depth_format, i::Usage::DEPTH_STENCIL_ATTACHMENT)?;
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
            device.create_image_view(&depth_image, depth_format, f::Swizzle::NO, DEPTH_RANGE.clone())?
        };

        let framebuffers = match &backbuffer {
            Backbuffer::Images(images) => {
                let device = device.lock().unwrap();

                let extent = d::Extent { width, height, depth: 1 };
                let pairs = images.iter()
                    .map(|image| {
                        let rtv = device.create_image_view(&image, surface_format, Swizzle::NO, COLOR_RANGE.clone())?;
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

        let ui_set_layout = {
            let device = device.lock().unwrap();
            device.create_descriptor_set_layout(&[])
        };

        let ui_pipeline_layout = {
            let device = device.lock().unwrap();
            device.create_pipeline_layout(Some(&set_layout), &[
                (ShaderStageFlags::VERTEX, 0..ModelLocals::SIZE),
            ])
        };

        let ui_render_pass = {
            let device = device.lock().unwrap();

            let attachment = pass::Attachment {
                format: Some(surface_format),
                ops: pass::AttachmentOps::new(pass::AttachmentLoadOp::Load, pass::AttachmentStoreOp::Store),
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

        let ui_pipeline = {
            let device = device.lock().unwrap();

            let shader = Shader::load_from_config(&config, &ShaderKey::new("ui"))?;
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
                    &ui_pipeline_layout,
                    subpass,
                );
                pipeline_desc.blender.targets.push(pso::ColorBlendDesc(pso::ColorMask::ALL, pso::BlendState::ALPHA));

                pipeline_desc.vertex_buffers.push(pso::VertexBufferDesc {
                    stride: mem::size_of::<UiVertex>() as u32,
                    rate: 0,
                });

                pipeline_desc.attributes.extend(UiVertex::desc());

                device.create_graphics_pipeline(&pipeline_desc)?
            };

            device.destroy_shader_module(vs_module);
            device.destroy_shader_module(fs_module);

            pipeline
        };

        let mut ui_desc_pool = {
            let device = device.lock().unwrap();

            device.create_descriptor_pool(1, &[])
        };

        let ui_desc_set = ui_desc_pool.allocate_set(&ui_set_layout);

        let ui_framebuffers = match &backbuffer {
            Backbuffer::Images(images) => {
                let device = device.lock().unwrap();

                let extent = d::Extent { width, height, depth: 1 };
                let pairs = images.iter()
                    .map(|image| {
                        let rtv = device.create_image_view(&image, surface_format, Swizzle::NO, COLOR_RANGE.clone())?;
                        Ok(rtv)
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                let fbos = pairs.iter()
                    .map(|rtv| device.create_framebuffer(&ui_render_pass, vec![rtv], extent))
                    .collect::<Result<Vec<_>, _>>().map_err(|_| RenderError::FramebufferCreation)?;

                fbos
            },
            Backbuffer::Framebuffer(_) => Err(RenderError::FramebufferCreation)?,
        };

        let viewport = command::Viewport {
            rect: command::Rect {
                x: 0,
                y: 0,
                w: width as _,
                h: height as _,
            },
            depth: 0.0 .. 1.0,
        };

        let locals = Buffer::<Locals, B>::new(device.clone(), 1, hal::buffer::Usage::UNIFORM, &memory_types).unwrap();

        {
            let device = device.lock().unwrap();
            device.write_descriptor_sets(vec![
                locals.descriptor_set(0, 0, &desc_set),
            ]);
        }

        let (frame_semaphore, frame_fence) = {
            let device = device.lock().unwrap();
            // TODO: remove fence
            (device.create_semaphore(), device.create_fence(false))
        };

        Ok(Self {
            command_pool,
            device,
            dimensions: (width, height),
            dpi_factor,
            frame_fence,
            frame_semaphore,
            _limits: limits,
            memory_types,
            // main shader
            pipeline_layout,
            render_pass,
            pipeline,
            desc_set,
            framebuffers,
            // ui shader
            ui_pipeline_layout,
            ui_render_pass,
            ui_pipeline,
            ui_desc_set,
            ui_framebuffers,
            //
            queue_group,
            viewport,
            swap_chain,
            //
            models: HashMap::new(),
            locals,
            ui: Vec::new(),
            //
            _instance: instance,
        })
    }

    pub fn load_model(&mut self, key: &mut ModelKey) -> &RLock<Model> {
        let model = match key.ty_mut() {
            ModelType::File(_) => unimplemented!(),
            ModelType::Procedural(procedural) => {
                let mut procedural = procedural.lock().unwrap();
                procedural.load(self.device.clone(), &self.memory_types[..])
            },
            ModelType::Quad => Model::quad([1.0, 0.0, 0.0], self.device.clone(), &self.memory_types[..]),
            ModelType::Hex => Model::hex([1.0, 0.0, 0.0], self.device.clone(), &self.memory_types[..]),
            ModelType::Sphere => Model::sphere([0.5, 0.5, 0.0], self.device.clone(), &self.memory_types[..]),
        };

        self.models.entry(key.clone())
            .or_insert(model)
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {

    }
}

impl<'a> System<'a> for Renderer {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, ModelKey>, ReadStorage<'a, ModelData>,
        Fetch<'a, Camera>,
        Fetch<'a, RLock<Map>>,
        FetchMut<'a, OpalUi>,
        Fetch<'a, WindowClosed>,
    );

    fn run(&mut self, (entities, mut model_keys, model_datas, camera, map, mut opal_ui, window_closed): Self::SystemData) {
        use specs::Join;

        if *window_closed == true {
            println!("Window Closed");
            return;
        }

        for model_key in (&mut model_keys).join() {
            match self.models.get_mut(model_key) {
                None => { self.load_model(model_key); },
                Some(_) => {
                    let reload = match model_key.ty_mut() {
                        ModelType::Procedural(procedural) => {
                            let mut procedural = procedural.lock().unwrap();
                            procedural.needs_reload()
                        },
                        _ => false,
                    };

                    if reload {
                        self.load_model(model_key);
                    }
                }
            };
        };

        let Self {
            device,
            dimensions,
            command_pool,
            frame_fence,
            frame_semaphore,
            locals,
            queue_group,
            swap_chain,
            viewport,
            //
            pipeline_layout,
            render_pass,
            pipeline,
            desc_set,
            framebuffers,
            //
            ui_pipeline_layout,
            ui_render_pass,
            ui_pipeline,
            ui_desc_set,
            ui_framebuffers,
            //
            ui,
            memory_types,
            ..
        } = self;

        let ratio = {
            let width = dimensions.0 as f32;
            let height = dimensions.1 as f32;
            width / height
        };

        locals.write(&[Locals {
            proj_view: camera.matrix(ratio).into(),
        }]).unwrap();

        command_pool.reset();
        let frame = swap_chain.acquire_frame(FrameSync::Semaphore(frame_semaphore));

        let mut command_buffer = command_pool.acquire_command_buffer(false);
        command_buffer.set_viewports(&[viewport.clone()]);
        command_buffer.set_scissors(&[viewport.rect]);
        command_buffer.bind_graphics_pipeline(pipeline);

        {
            let mut encoder = command_buffer.begin_render_pass_inline(
                &render_pass,
                &framebuffers[frame.id()],
                viewport.rect,
                &[
                    command::ClearValue::Color(command::ClearColor::Float([0.8, 0.8, 0.8, 1.0])),
                    command::ClearValue::DepthStencil(command::ClearDepthStencil(1.0, 0)),
                ],
            );
            encoder.bind_graphics_descriptor_sets(pipeline_layout, 0, Some(desc_set)); //TODO

            let map = map.read().unwrap();

            for (entity, model_key) in (&*entities, &model_keys).join() {
                // this unwrap is safe because all the models are added at the top of the function.
                let model = self.models.get(model_key).unwrap();
                let model = model.read().unwrap();

                let locals = {
                    let model_data = match model_datas.get(entity) {
                        Some(data) => *data,
                        None => Default::default(),
                    };

                    let position = match map.location(&entity) {
                        Some(position) => *position,
                        None => Vector3::new(0, 0, 0),
                    };

                    let model_data = model_data.to_matrix(&position);

                    ModelLocals {
                        model: model_data.into(),
                    }
                };

                encoder.push_graphics_constants(
                    pipeline_layout,
                    ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT,
                    0,
                    &locals.data()[..],
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

        let mut index = 0;
        let mut vertices = vec![];
        let mut indices = vec![];

        #[derive(Copy, Clone, Debug, PartialEq, Eq)]
        enum UiState { None, Plain, Image, Text };

        let mut current_state = UiState::None;

        let vx = |x: f64| {
            if ratio > 1.0 {
                (x as f32) * (dimensions.1 as f32 / dimensions.0 as f32)
            } else {
                x as f32
            }
        };

        let vy = |y: f64| {
            if ratio < 1.0 {
                (y as f32) * (dimensions.0 as f32 / dimensions.1 as f32)
            } else {
                y as f32
            }
        };

        let finish_state = |vertices: &mut Vec<UiVertex>, indices: &mut Vec<u32>| {
            if vertices.is_empty() || indices.is_empty() {
                return None;
            }

            let mut vertex_buffer = Buffer::<UiVertex, B>::new(device.clone(), vertices.len() as u64, hal::buffer::Usage::VERTEX, &memory_types).unwrap();
            vertex_buffer.write(&vertices[..]).unwrap();

            let mut index_buffer = Buffer::<u32, B>::new(device.clone(), indices.len() as u64, hal::buffer::Usage::INDEX, &memory_types).unwrap();
            index_buffer.write(&indices[..]).unwrap();

            vertices.clear();
            indices.clear();

            Some(Model { vertex_buffer, index_buffer })
        };

        if opal_ui.is_some() {
            let opal_ui = opal_ui.as_mut().unwrap();
            let mut opal_ui = opal_ui.walk();

            ui.clear();

            while let Some(primitive) = opal_ui.next_primitive() {
                let render::Primitive { kind, rect, .. } = primitive;

                match kind {
                    render::PrimitiveKind::Rectangle { color } => {
                        if current_state != UiState::Plain {
                            finish_state(&mut vertices, &mut indices)
                                .map(|m| ui.push(m));
                            index = 0;
                            current_state = UiState::Plain;
                        }

                        let (l, r, b, t) = rect.l_r_b_t();
                        let v = |x, y| {
                            UiVertex {
                                position: [vx(x), vy(y)],
                                color: color.to_fsa(),
                            }
                        };

                        // Bottom left triangle.
                        vertices.push(v(l, t));
                        vertices.push(v(r, b));
                        vertices.push(v(l, b));

                        // Top right triangle.
                        vertices.push(v(l, t));
                        vertices.push(v(r, b));
                        vertices.push(v(r, t));

                        indices.push(index);
                        indices.push(index + 1);
                        indices.push(index + 2);
                        indices.push(index + 3);
                        indices.push(index + 4);
                        indices.push(index + 5);
                        index += 6;
                    },
                    render::PrimitiveKind::TrianglesSingleColor { color, triangles } => {
                        if triangles.is_empty() {
                            continue;
                        }

                        if current_state != UiState::Plain {
                            finish_state(&mut vertices, &mut indices)
                                .map(|m| ui.push(m));
                            index = 0;
                            current_state = UiState::Plain;
                        }

                        let v = |p: [f64; 2]| {
                            UiVertex {
                                position: [vx(p[0]), vy(p[1])],
                                color: color.into(),
                            }
                        };

                        for triangle in triangles {
                            vertices.push(v(triangle[0]));
                            vertices.push(v(triangle[1]));
                            vertices.push(v(triangle[2]));

                            indices.push(index);
                            indices.push(index + 1);
                            indices.push(index + 2);
                            index += 3;
                        }
                    }
                    render::PrimitiveKind::TrianglesMultiColor { triangles } => {
                        if triangles.is_empty() {
                            continue;
                        }

                        if current_state != UiState::Plain {
                            finish_state(&mut vertices, &mut indices)
                                .map(|m| ui.push(m));
                            index = 0;
                            current_state = UiState::Plain;
                        }

                        let v = |(p, c): ([f64; 2], conrod::color::Rgba)| {
                            UiVertex {
                                position: [vx(p[0]), vy(p[1])],
                                color: c.into(),
                            }
                        };

                        for triangle in triangles {
                            vertices.push(v(triangle[0]));
                            vertices.push(v(triangle[1]));
                            vertices.push(v(triangle[2]));

                            indices.push(index);
                            indices.push(index + 1);
                            indices.push(index + 2);
                            index += 3;
                        }
                    }
                    _ => {
                        println!("Unsupported");
                        println!("index: {}", indices.len());
                        current_state = UiState::None;
                        index = 0;
                    },
                }
            }
        }

        if current_state != UiState::None {
            finish_state(&mut vertices, &mut indices)
                .map(|m| ui.push(m));
            index = 0;
        }

        if ui.is_empty() == false {
            command_buffer.bind_graphics_pipeline(ui_pipeline);

            let mut encoder = command_buffer.begin_render_pass_inline(
                &ui_render_pass,
                &ui_framebuffers[frame.id()],
                viewport.rect,
                &[],
            );
            encoder.bind_graphics_descriptor_sets(ui_pipeline_layout, 0, Some(ui_desc_set)); //TODO

            for model in ui {
                let locals = {
                    let model_data: ModelData = Default::default();
                    let position = Vector3::new(0, 0, 0);
                    let model_data = model_data.to_matrix(&position);

                    ModelLocals {
                        model: model_data.into(),
                    }
                };

                encoder.push_graphics_constants(
                    pipeline_layout,
                    ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT,
                    0,
                    &locals.data()[..],
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

        let submit = command_buffer.finish();
        let submission = Submission::new()
            .wait_on(&[(&*frame_semaphore, PipelineStage::BOTTOM_OF_PIPE)])
            .submit(Some(submit));

        let mut queue = &mut queue_group.queues[0];
        queue.submit(submission, Some(frame_fence));

        {
            let device = device.lock().unwrap();
            // TODO: replace with semaphore
            device.wait_for_fence(&frame_fence, !0);
        }

        swap_chain.present(&mut queue, &[]);
    }
}
