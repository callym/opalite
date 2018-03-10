use std::{ collections::HashMap, iter::FromIterator, sync::{ Arc, Mutex } };
use back::{ self, Backend as B };
use gfx::{ self, pso::GraphicsPipelineData, Device };
// gfx traits!
use hal::{ self, command, Instance as _Instance, PhysicalDevice as _PhysicalDevice };
use specs::{ Entities, Fetch, ReadStorage, System, VecStorage, WriteStorage };
use winit::Window;
use crate::{ Config, Position, WindowClosed };

pub type ColorFormat = gfx::format::Rgba8Srgb;
pub type Allocator = gfx::allocators::StackAllocator<B>;

mod allocators;
pub use self::allocators::Allocators;

mod framebuffers;
pub use self::framebuffers::Framebuffers;

mod model;
pub use self::model::{ ModelKey, Model, ModelType, Vertex };

mod pipeline;
pub use self::pipeline::{ desc, Pipeline };

mod shader;
pub use self::shader::{ ShaderKey, Shader };

pub struct Renderer {
    allocators: Allocators,
    context: gfx::Context<B, hal::Graphics>,
    device: Arc<Mutex<Device<B>>>,
    models: HashMap<ModelKey, Model>,
    pipelines: HashMap<ShaderKey, Pipeline>,
    // instance has to be dropped after everything else
    instance: back::Instance,
}

impl Renderer {
    pub fn new(config: Config, window: &Window) -> Self {
        let instance = back::Instance::create("Opalite", 1);
        let surface = instance.create_surface(&window);

        let adapter = {
            let mut adapters = instance.enumerate_adapters();
            for adapter in &adapters {
                println!("{:?}", adapter.info);
            }
            adapters.remove(0)
        };
        let limits = adapter.physical_device.get_limits();

        let (context, backbuffers) = gfx::Context::<B, hal::Graphics>::init::<ColorFormat>(surface, adapter).unwrap();
        let backbuffers = Arc::new(backbuffers);
        let mut device = (*context.ref_device()).clone();
        let (desc, desc_data): (desc::Set<B>, desc::Data<B>) = device.create_descriptors(1).pop().unwrap();
        let (desc, desc_data) = (Arc::new(desc), Arc::new(desc_data));
        let device = Arc::new(Mutex::new(device));

        let pipelines = config.shaders.keys()
            .map(|k| {
                let k = k.clone();
                let shader = Shader::load_from_config(&config, &k).unwrap();
                (k, Pipeline::new(&window, &context, limits, device.clone(), desc.clone(), desc_data.clone(), shader, backbuffers.clone()))
            })
            .collect::<HashMap<_, _>>();

        let allocators = Allocators::new(device.clone(), limits);

        Self {
            allocators,
            context,
            device,
            instance,
            models: HashMap::new(),
            pipelines,
        }
    }

    pub fn load_model(&mut self, key: &ModelKey) -> &Model {
        let mut device = self.device.lock().unwrap();
        let mut encoder_pool = self.context.acquire_encoder_pool();
		let mut encoder = encoder_pool.acquire_encoder();

        let model = match key.ty() {
            ModelType::File(_) => unimplemented!(),
            ModelType::Quad => Model::quad(&mut device, &mut encoder, &mut self.allocators.upload),
        };

        self.models.insert(key.clone(), model);
        self.models.get(&key).unwrap()
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

        let pipeline = self.pipelines.get(&ShaderKey::new("main".into())).unwrap();
        let framebuffers = pipeline.framebuffers();

        let frame = self.context.acquire_frame();
        let mut encoder_pool = self.context.acquire_encoder_pool();
        let mut encoder = encoder_pool.acquire_encoder();

        let (backbuffer, framebuffer, frame_rtv) = framebuffers.get_frame_resources(frame.id());

        encoder.clear_color(&backbuffer.color, command::ClearColor::Float([1.0; 4]));

        for (position, model_key) in (&positions, &model_keys).join() {
            // this unwrap is safe because all the models are added at the top of the function.
            let model = self.models.get(model_key).unwrap();

            let data = pipeline::pipe::Data {
                desc: pipeline.descriptors(),
                color: frame_rtv,
                vertices: &model.vertex_buffer,
                viewports: &[framebuffers.viewport()],
                scissors: &[framebuffers.scissor()],
                framebuffer,
            };

            let mut data = data.begin_renderpass(&mut encoder, &pipeline.pipeline());
            data.bind_index_buffer(hal::buffer::IndexBufferView {
                buffer: &model.index_buffer.as_ref().resource(),
                offset: 0,
                index_type: hal::IndexType::U32,
            });
            data.draw_indexed(0..model.index_count, 0, 0..1);
        }

        self.context.present(vec![encoder.finish()]);
    }
}
