use std::{
    collections::{ HashMap, HashSet },
    ops::DerefMut,
    sync::{ Arc, Mutex },
};
use fibers::sync::oneshot::Receiver;
use futures::{
    prelude::*,
    future::*,
};
use crate::{
    Component,
    Handler,
    Id,
    Message,
    Store,
};

pub struct Tile {
    pub x: i64,
    pub y: i64,
    pub z: i64,
    pub entities: HashSet<Id>,
}

impl Tile {
    pub fn coordinates(&self) -> (i64, i64, i64) {
        (self.x, self.y, self.z)
    }

    pub fn entities_iter(&self) -> impl Iterator<Item = &Id> {
        self.entities.iter()
    }
}

impl Component for Tile { }

pub struct MoveMessage {
    pub id: Id,
    pub coordinates: (i64, i64, i64),
}

impl Message for MoveMessage { }

pub struct MapStore {
    store: HashMap<Id, Arc<Mutex<Tile>>>,
}

impl MapStore {
    pub fn new(x: i64, y: i64, z: i64) -> Self {
        let mut store = HashMap::new();

        for x in 0..x {
            for y in 0..y {
                for z in 0..z {
                    store.insert(Id::new(), Arc::new(Mutex::new(Tile {
                        x, y, z,
                        entities: HashSet::new(),
                    })));
                }
            }
        }

        Self { store }
    }

    pub fn get_tile_at(&self, coordinates: (i64, i64, i64)) -> Option<Arc<Mutex<Tile>>> {
        for tile_arc in self.get_all() {
            let tile = tile_arc.lock().unwrap();
            if tile.coordinates() == coordinates {
                return Some(tile_arc.clone());
            }
        }

        None
    }

    pub fn do_tile_at<R>(&self, coordinates: (i64, i64, i64), fun: impl FnOnce(&mut Tile) -> R) {
        let tile = self.get_tile_at(coordinates);
        if let Some(tile) = tile {
            let mut tile = tile.lock().unwrap();
            fun(&mut *tile);
        }
    }

    pub fn get_tile_with(&self, id: Id) -> Option<Arc<Mutex<Tile>>> {
        for tile_arc in self.get_all() {
            let tile = tile_arc.lock().unwrap();
            if tile.entities.contains(&id) {
                return Some(tile_arc.clone());
            }
        }

        None
    }

    pub fn do_tile_with<R>(&self, id: Id, fun: impl FnOnce(&mut Tile) -> R) {
        let tile = self.get_tile_with(id);
        if let Some(tile) = tile {
            let mut tile = tile.lock().unwrap();
            fun(&mut *tile);
        }
    }
}

impl Handler<MoveMessage, (), ()> for MapStore {
    fn send(&mut self, message: MoveMessage) -> Box<Future<Item = (), Error = ()> + Send> {
        let MoveMessage { id, coordinates } = message;

        match self.get_tile_at(coordinates) {
            Some(_) => {
                // do collision checks here
                let can_move = true;

                if can_move == false {
                    return Box::new(err(()));
                }
            },
            None => return Box::new(err(())),
        };

        let old_tile = match self.get_tile_with(id) {
            Some(tile) => tile,
            None => return Box::new(err(())),
        };

        let new_tile = match self.get_tile_at(coordinates) {
            Some(tile) => tile,
            None => return Box::new(err(())),
        };

        Box::new(lazy(move || {
            let mut old_tile = old_tile.lock().unwrap();
            let mut new_tile = new_tile.lock().unwrap();

            old_tile.entities.remove(&id);
            new_tile.entities.insert(id);

            ok(())
        }))
    }
}

impl Store for MapStore {
    type Component = Tile;

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
