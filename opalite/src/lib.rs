#![feature(
    arbitrary_self_types,
    const_fn,
    crate_in_paths,
    fs_read_write,
    get_type_id,
    macro_at_most_once_rep,
    match_default_bindings,
    nll,
    specialization,
)]

#[macro_use] extern crate opalite_macro;

extern crate anymap;
pub extern crate bincode;
// rust says that macros from cgmath aren't used even though they are.
#[macro_use] pub extern crate cgmath;
#[macro_use] pub extern crate conrod;
#[macro_use] extern crate failure;
#[macro_use] extern crate failure_derive;
extern crate genmesh;
extern crate glsl_to_spirv;
pub extern crate gfx_hal as hal;
extern crate gfx_backend_dx12 as back;
extern crate gltf;
extern crate gltf_importer;
extern crate gltf_utils;
extern crate gluon;
#[macro_use] extern crate gluon_vm;
extern crate image;
extern crate ordered_float;
extern crate owning_ref;
extern crate ron;
extern crate rusttype;
#[macro_use] extern crate serde;
pub extern crate specs;
#[macro_use] pub extern crate specs_derive;
extern crate uuid;
extern crate winit;
extern crate zip;

mod ai;
mod config;
#[macro_use] pub mod gluon_api;
mod input_events;
mod map;
mod mutex_ext;
mod opal;
mod picker;
pub mod renderer;
mod resources;
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
};

pub use gluon_api::{
    Data,
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
    OpalUi,
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
    Light,
    LightType,
    MaterialDesc,
    Model,
    ModelData,
    ModelKey,
    ModelType,
    ProceduralModel,
    ShaderKey,
    SurfaceType,
    Vertex,
};

pub use resources::Resources;

pub use system::{
    Message,
    MessageIter,
    MessageQueue,
    MessageReceiver,
    MessageSender,
    Shard,
};
