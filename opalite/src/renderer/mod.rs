use std::{ collections::HashMap, path::PathBuf };
use back::{ self, Backend as B };
use failure::{ self, Error };
use glsl_to_spirv::{ self, ShaderType, SpirvOutput };
use gfx::{
    self,
    allocators::StackAllocator as Allocator,
    format::Rgba8Srgb as ColorFormat,
};
// gfx traits!
use hal::{
    self,
    Device as _Device,
    Instance as _Instance,
    PhysicalDevice as _PhysicalDevice,
    Primitive as _Primitive
};
use specs::{ Entities, FetchMut, ReadStorage, System, VecStorage, WriteStorage };
use winit::Window;
use crate::{ Config, ShaderLocation };

mod shader;
pub use self::shader::{ ShaderKey, Shader };

pub struct Renderer {
    config: Config,
    shaders: HashMap<ShaderKey, Shader>,
    window: Window,
}

impl Renderer {
    pub fn new(config: Config, window: Window) -> Self {
        let (width, height) = window.get_inner_size().unwrap();

        let instance = back::Instance::create("Opalite", 1);
        let surface = instance.create_surface(&window);

        let adapter = {
            let mut adapters = instance.enumerate_adapters();
            for adapter in &adapters {
                println!("{:?}", adapter.info);
            }
            adapters.remove(0)
        };
        let limits = adapter.physical_device.limits();

        let (mut context, backbuffers) = gfx::Context::<B, hal::Graphics>::init::<ColorFormat>(surface, adapter).unwrap();
        let mut device = (*context.ref_device()).clone();

        let mut renderer = Self {
            config,
            shaders: HashMap::new(),
            window,
        };

        renderer.shaders = renderer.config.shaders.keys()
            .map(|k| (k.clone(), renderer.load_shader(k).unwrap()))
            .collect::<HashMap<_, _>>();

        renderer
    }

    pub fn load_shader(&self, shader: &ShaderKey) -> Result<Shader, Error> {
        ensure!(self.config.shaders.contains_key(&shader), "Shader isn't in Opal.ron");

        let path = match self.config.shaders.get(&shader).unwrap() {
            ShaderLocation::Builtin(path) => {
                let mut base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                base.push(path);
                base
            },
            ShaderLocation::Custom(path) => {
                let mut base = PathBuf::from(::std::env::var("CARGO_MANIFEST_DIR").unwrap());
                base.push(path);
                base
            },
        };

        Shader::load(&path)
    }
}

impl<'a> System<'a> for Renderer {
    type SystemData = ();

    fn run(&mut self, data: Self::SystemData) {

    }
}
