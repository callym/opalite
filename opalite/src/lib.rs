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
extern crate fibers;
#[macro_use] extern crate futures;
extern crate uuid;

pub use futures::Future;

mod component_store;
mod game_loop;
mod map;
mod message;
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
    Id,
    Store,
    HashStore,
};
