use back::{ self, Backend as B };
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
use crate::{ Config };

mod shader;
pub use self::shader::{ ShaderKey };

pub struct Renderer {
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

        Self {
            window,
        }
    }
}

impl<'a> System<'a> for Renderer {
    type SystemData = ();

    fn run(&mut self, data: Self::SystemData) {

    }
}
