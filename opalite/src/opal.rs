use specs::{ DispatcherBuilder, Dispatcher, World };
use crate::{ AiComponent, AiSystem, Position, MapSystem };

use std::sync::{ Arc, Mutex, mpsc };

#[derive(Clone)]
pub struct MessageQueue<T>(Arc<Mutex<mpsc::Sender<T>>>);

impl<T> MessageQueue<T> {
    pub fn new(queue: mpsc::Sender<T>) -> Self {
        MessageQueue(Arc::new(Mutex::new(queue)))
    }

    pub fn send(&mut self, message: T) {
        let sender = self.0.lock().unwrap();
        sender.send(message).unwrap();
    }
}

pub struct DefaultSystems {
    ai_system: AiSystem,
    map_system: MapSystem,
}

pub struct Opal<'a, 'b> {
    dispatcher: Dispatcher<'a, 'b>,
    world: World,
}

impl<'a, 'b> Opal<'a, 'b> {
    pub fn default_systems() -> DefaultSystems {
        DefaultSystems {
            ai_system: AiSystem::new(),
            map_system: MapSystem::new(),
        }
    }

    pub fn default_dispatcher<'c, 'd>(systems: DefaultSystems) -> DispatcherBuilder<'c, 'd> {
        DispatcherBuilder::new()
            .add(systems.map_system, "MapSystem", &[])
            .add(systems.ai_system, "AiSystem", &["MapSystem"])
    }

    pub fn default_world(systems: &DefaultSystems) -> World {
        let mut world = World::new();

        world.register::<AiComponent>();
        world.register::<Position>();

        world.add_resource(systems.map_system.sender());

        world
    }

    pub fn new<'c, 'd>(world: World, dispatcher_builder: DispatcherBuilder<'c, 'd>) -> Opal<'c, 'd> {
        Self {
            dispatcher: dispatcher_builder.build(),
            world: world,
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn run(&mut self) -> Result<(), ()> {
        self.dispatcher.dispatch(&mut self.world.res);

        Ok(())
    }
}
