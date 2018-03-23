#![feature(
    arbitrary_self_types,
    conservative_impl_trait,
    const_fn,
    crate_in_paths,
    get_type_id,
    match_default_bindings,
    nll,
    specialization,
    universal_impl_trait,
)]

#[macro_use] extern crate opalite_macro;

pub extern crate bincode;
#[macro_use] pub extern crate cgmath;
#[macro_use] extern crate failure;
#[macro_use] extern crate failure_derive;
extern crate futures;
extern crate glsl_to_spirv;
pub extern crate gfx_hal as hal;
extern crate gfx_backend_vulkan as back;
extern crate owning_ref;
extern crate ron;
#[macro_use] extern crate serde;
pub extern crate specs;
#[macro_use] pub extern crate specs_derive;
extern crate uuid;
extern crate winit;

mod ai;
mod config;
mod input_events;
mod map;
mod mutex_ext;
mod opal;
mod picker;
pub mod renderer;
mod system;

pub use back::Backend;

pub use ai::{
    AiComponent,
    AiGoalDo,
    AiGoal,
    AiSystem,
};

pub use config::{
    Config,
    ConfigBuilder,
    ShaderLocation,
};

pub use input_events::{
    InputEvent,
    InputEventType,
    InputEventHandler,
};

pub use map::{
    CollisionLayer,
    CollisionLayers,
    InitialPosition,
    Position,
    Map,
    MapMessage,
    MapSystem,
};

pub use mutex_ext::{ RLock, WLock };

pub use opal::{
    OpalBuilder,
    Opal,
    WindowClosed,
};

pub use picker::{
    PickerSystem,
};

pub use renderer::{
    Buffer,
    BufferData,
    Camera,
    Renderer,
    Model,
    ModelData,
    ModelKey,
    ModelType,
    ProceduralModel,
    ShaderKey,
    Vertex,
};

pub use system::{
    Message,
    MessageIter,
    MessageQueue,
    MessageReceiver,
    MessageSender,
    Shard,
};
