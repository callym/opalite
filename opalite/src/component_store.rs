use std::sync::{ Arc, Mutex };
use anymap::AnyMap;
use crate::{
    Id,
    MapStore,
    Store,
};

pub struct ComponentStores {
    store: AnyMap,
}

impl ComponentStores {
    pub fn new() -> Self {
        Self {
            store: AnyMap::new(),
        }
    }

    pub fn new_with_default_components(x: i64, y: i64, z: i64) -> Self {
        let mut stores = Self::new();

        stores.register(MapStore::new(x, y, z));

        stores
    }

    pub fn get<S: 'static + Store>(&self) -> Option<&S> {
        self.store.get()
    }

    pub fn get_mut<S: 'static + Store>(&mut self) -> Option<&mut S> {
        self.store.get_mut()
    }

    pub fn do_with<S: 'static + Store, R>(&mut self, fun: impl FnOnce(&mut S) -> R) -> Option<R> {
        match self.get_mut::<S>() {
            Some(store) => Some(fun(store)),
            None => None,
        }
    }

    pub fn register(&mut self, store: impl Store + 'static) {
        self.store.insert(store);
    }

    pub fn get_component<S: 'static + Store>(&self, id: &Id) -> Option<Arc<Mutex<S::Component>>> {
        match self.get::<S>() {
            Some(store) => store.get(id),
            None => None,
        }
    }

    pub fn do_with_component<S: 'static + Store, R>(&self, id: &Id, fun: impl FnOnce(&mut S::Component) -> R) -> Option<R> {
        match self.get::<S>() {
            Some(store) => store.do_with(id, fun),
            None => None,
        }
    }
}
