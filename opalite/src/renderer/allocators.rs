use std::sync::{ Arc, Mutex };
use back::Backend as B;
use hal::Limits;
use gfx::{ self, Device };
use crate::renderer::Allocator;

pub struct Allocators {
    pub data: Allocator,
    pub upload: Allocator,
    device: Arc<Mutex<Device<B>>>,
}

impl Allocators {
    pub fn new(device_arc: Arc<Mutex<Device<B>>>, limits: Limits) -> Self {
        let device = device_arc.lock().unwrap();

        let data = Allocator::new(
            gfx::memory::Usage::Data,
            &device,
            limits,
        );

        let upload = Allocator::new(
            gfx::memory::Usage::Upload,
            &device,
            limits,
        );

        Self { data, upload, device: device_arc.clone(), }
    }
}
