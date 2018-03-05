use std::{
    collections::HashMap,
    default::Default,
    sync::{ Arc, Mutex },
};
use uuid::Uuid;
use crate::{
    GameLoop,
    Handler,
    Message,
};

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct Id(Uuid);

impl Id {
    pub fn new() -> Self {
        Id(Uuid::new_v4())
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Component { }

pub trait Store {
    type Component: Component + 'static;

    fn has(&self, id: &Id) -> bool;

    fn get(&self, id: &Id) -> Option<Arc<Mutex<Self::Component>>>;
    fn get_all(&self) -> Vec<Arc<Mutex<Self::Component>>>;

    fn do_with<R>(&self, id: &Id, fun: impl FnOnce(&mut Self::Component) -> R) -> Option<R> {
        let component = self.get(id);
        if let Some(component) = component {
            let mut component = component.lock().unwrap();
            Some(fun(&mut *component))
        } else {
            None
        }
    }

    fn set(&mut self, id: Id, component: Self::Component);

    fn send_message<M, R: 'static, E: 'static>(&mut self, id: &Id, message: M, game_loop: &mut GameLoop) -> Result<(), ()> where Self::Component: Handler<M, R, E>, M: Message {
        let component = self.get(id).ok_or(Err(())?)?;
        let future = message.send(component);
        game_loop.spawn(future);
        Ok(())
    }
}

pub struct HashStore<C: Component> {
    store: HashMap<Id, Arc<Mutex<C>>>,
}

impl<C> HashStore<C> where C: Component {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }
}

impl<C> Store for HashStore<C> where C: Component + 'static {
    type Component = C;

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
