#![feature(
    arbitrary_self_types,
    conservative_impl_trait,
    crate_in_paths,
    get_type_id,
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
extern crate uuid;
extern crate winit;

pub use futures::{ Future, FutureExt };
pub use winit::WindowBuilder;

mod component_store;
mod game_loop;
mod map;
mod message;
mod render;
mod store;

pub use component_store::ComponentStores;

pub use game_loop::GameLoop;

pub use map::{
    MapStore,
    MoveMessage,
    Tile,
};

pub use message::{
    Handler,
    Message,
};

pub use store::{
    Component,
    ComponentIter,
    Id,
    Store,
    HashStore,
};
