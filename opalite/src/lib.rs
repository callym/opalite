#![feature(
    arbitrary_self_types,
    conservative_impl_trait,
    crate_in_paths,
    get_type_id,
    match_default_bindings,
    nll,
    specialization,
    universal_impl_trait,
)]

extern crate anymap;
extern crate failure;
#[macro_use] extern crate futures;
extern crate gfx_hal as hal;
extern crate gfx_backend_vulkan as back;
#[macro_use] extern crate gfx_render as gfx;
pub extern crate specs;
#[macro_use] pub extern crate specs_derive;
extern crate uuid;
extern crate winit;

pub use futures::{ Future, FutureExt };
pub use winit::WindowBuilder;

mod ai;
mod map;
mod opal;

pub use ai::{
    AiComponent,
    AiGoal,
    AiSystem,
};

pub use map::{
    Position,
    MapMessage,
    MapSystem,
};

pub use opal::{ MessageQueue, Opal };
