use std::{
    collections::{ HashMap, HashSet },
    ops::DerefMut,
    sync::{ Arc, Mutex },
};
use futures::{
    prelude::*,
    channel::oneshot::Receiver,
    future::*,
};
use hal::{ Instance as _Instance, PhysicalDevice as _PhysicalDevice, };
use back::{ Instance, };
use gfx;
use winit::{ EventsLoop, Window, WindowBuilder };

use crate::{
    Component,
    Handler,
    Id,
    Message,
    Store,
};

pub struct RenderData;

impl Component for RenderData { }

pub struct RenderMessage;

impl Message for RenderMessage { }

pub struct RenderStore {
    window: Window,
    store: HashMap<Id, Arc<Mutex<RenderData>>>,
}

impl RenderStore {
    pub fn new(title: &str, window: Window) -> Self {
        let instance = Instance::create(title, 1);
        let surface = instance.create_surface(&window);
        let mut adapters = instance.enumerate_adapters();
        for adapter in &adapters {
            println!("{:?}", adapter.info);
        }
        let adapter = adapters.remove(0);
        let limits = adapter.physical_device.limits();

        Self {
            window,
            store: HashMap::new(),
        }
    }
}

impl Handler<RenderMessage, (), ()> for RenderStore {
    fn send(&mut self, _: RenderMessage) -> Box<Future<Item = (), Error = ()> + Send> {
        Box::new(ok(()))
    }
}

impl Store for RenderStore {
    type Component = RenderData;

    fn has(&self, id: &Id) -> bool {
        self.store.contains_key(id)
    }

    fn get(&self, id: &Id) -> Option<Arc<Mutex<Self::Component>>> {
        self.store.get(id).map(|v| v.clone())
    }

    fn get_all(&self) -> Vec<Arc<Mutex<Self::Component>>> {
        self.store.iter().map(|(_, v)| v.clone()).collect()
    }

    fn set(&mut self, id: Id, component: Self::Component) {
        self.store.insert(id, Arc::new(Mutex::new(component)));
    }
}
