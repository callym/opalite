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
#[macro_use] extern crate failure;
extern crate futures;
extern crate glsl_to_spirv;
extern crate gfx_hal as hal;
extern crate gfx_backend_vulkan as back;
#[macro_use] extern crate gfx_render as gfx;
extern crate owning_ref;
extern crate ron;
#[macro_use] extern crate serde;
pub extern crate specs;
#[macro_use] pub extern crate specs_derive;
extern crate uuid;
extern crate winit;

mod ai;
mod config;
mod map;
mod opal;
mod renderer;
mod system;

pub use ai::{
    AiComponent,
    AiGoal,
    AiSystem,
};

pub use config::{
    Config,
    ConfigBuilder,
    ShaderLocation,
};

pub use map::{
    Position,
    MapMessage,
    MapSystem,
};

pub use opal::{
    OpalBuilder,
    Opal,
    WindowClosed,
};

pub use renderer::{
    Renderer,
    ModelKey,
    ModelType,
    ShaderKey,
};

pub use system::{
    Message,
    MessageIter,
    MessageQueue,
    MessageReceiver,
    MessageSender,
    Shard,
};
