use std::{ ops::Drop, sync::{ Arc, Mutex } };
use cgmath::Vector3;
use failure::Error;
use specs::{ Entities, Fetch, FetchMut, ReadStorage, System, WriteStorage };
use winit::Window;
use crate::{ Config, Map, OpalUi, RLock, WindowClosed };

use back;
use back::Backend as B;

use hal;
use hal::{ format as f, image as i, pool };
use hal::{ Backend, Device, Instance, PhysicalDevice, Surface, Swapchain };
use hal::{
    Adapter,
    FrameSync,
    SwapchainConfig,
};
use hal::format::ChannelType;
use hal::pso::PipelineStage;
use hal::queue::Submission;

mod buffer;
pub use self::buffer::{ Buffer, BufferData };

mod camera;
pub use self::camera::Camera;

pub mod model;
pub use self::model::{ ModelKey, Model, ModelData, ModelType, ProceduralModel, Vertex, UiVertex };

mod pipe;
pub use self::pipe::{
    MainPipe, MainLocals, MainModelLocals,
    UiPipe,
    Pipe,
};

mod push_constant;
pub use self::push_constant::PushConstant;

mod shader;
pub use self::shader::{ ShaderKey, Shader };

const COLOR_RANGE: i::SubresourceRange = i::SubresourceRange {
    aspects: f::Aspects::COLOR,
    levels: 0 .. 1,
    layers: 0 .. 1,
};

const DEPTH_RANGE: i::SubresourceRange = i::SubresourceRange {
    aspects: f::Aspects::DEPTH,
    levels: 0 .. 1,
    layers: 0 .. 1,
};

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
    swap_chain: <B as Backend>::Swapchain,
    main_pipe: MainPipe,
    ui_pipe: UiPipe,
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

        let (frame_semaphore, frame_fence) = {
            let device = device.lock().unwrap();
            // TODO: remove fence
            (device.create_semaphore(), device.create_fence(false))
        };

        let main_pipe = pipe::MainPipe::new(
            &backbuffer,
            &config,
            (width, height),
            dpi_factor,
            device.clone(),
            &memory_types[..],
            surface_format,
            Some(depth_format),
        )?;

        let ui_pipe = pipe::UiPipe::new(
            &backbuffer,
            &config,
            (width, height),
            dpi_factor,
            device.clone(),
            &memory_types[..],
            surface_format,
            None,
        )?;

        Ok(Self {
            command_pool,
            device,
            dimensions: (width, height),
            dpi_factor,
            frame_fence,
            frame_semaphore,
            _limits: limits,
            memory_types,
            queue_group,
            swap_chain,
            _instance: instance,
            main_pipe,
            ui_pipe,
        })
    }

    pub fn load_model(&mut self, key: &mut ModelKey) -> RLock<Model> {
        match key.ty_mut() {
            ModelType::File(_) => unimplemented!(),
            ModelType::Procedural(procedural) => {
                let mut procedural = procedural.lock().unwrap();
                procedural.load(self.device.clone(), &self.memory_types[..])
            },
            ModelType::Quad => Model::quad([1.0, 0.0, 0.0], self.device.clone(), &self.memory_types[..]),
            ModelType::Hex => Model::hex([1.0, 0.0, 0.0], self.device.clone(), &self.memory_types[..]),
            ModelType::Sphere => Model::sphere([0.5, 0.5, 0.0], self.device.clone(), &self.memory_types[..]),
        }
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
            match self.main_pipe.models_mut().get_mut(model_key) {
                None => {
                    let model = self.load_model(model_key);
                    self.main_pipe.models_mut().insert(model_key.clone(), model);
                },
                Some(_) => {
                    let reload = match model_key.ty_mut() {
                        ModelType::Procedural(procedural) => {
                            let mut procedural = procedural.lock().unwrap();
                            procedural.needs_reload()
                        },
                        _ => false,
                    };

                    if reload {
                        let model = self.load_model(model_key);
                        self.main_pipe.models_mut().insert(model_key.clone(), model);
                    }
                }
            }
        };

        let Self {
            device,
            dimensions,
            command_pool,
            frame_fence,
            frame_semaphore,
            queue_group,
            swap_chain,
            main_pipe,
            ui_pipe,
            //
            memory_types,
            ..
        } = self;

        let ratio = {
            let width = dimensions.0 as f32;
            let height = dimensions.1 as f32;
            width / height
        };

        command_pool.reset();
        let frame = swap_chain.acquire_frame(FrameSync::Semaphore(frame_semaphore));

        let mut command_buffer = command_pool.acquire_command_buffer(false);

        let models = (&*entities, &model_keys).join()
            .map(|(entity, model_key)| {
                let map = map.read().unwrap();

                let model_data = match model_datas.get(entity) {
                    Some(data) => *data,
                    None => Default::default(),
                };

                let position = match map.location(&entity) {
                    Some(position) => *position,
                    None => Vector3::new(0, 0, 0),
                };

                let model_data = model_data.to_matrix(&position);

                (
                    model_key,
                    MainModelLocals {
                        model: model_data.into(),
                    }
                )
            })
            .collect::<Vec<_>>();

        main_pipe.update_locals(MainLocals {
            proj_view: camera.matrix(ratio).into(),
        });

        main_pipe.draw(
            &mut command_buffer,
            frame.id(),
            &models[..],
        );

        ui_pipe.draw(
            &mut command_buffer,
            &memory_types[..],
            frame.id(),
            &mut opal_ui,
        );

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
