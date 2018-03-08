use specs::{ DispatcherBuilder, Dispatcher, World };
use crate::{ AiComponent, AiSystem, Position, MapSystem, Shard };

pub struct DefaultSystems {
    ai_system: Option<AiSystem>,
    map_system: Option<MapSystem>,
}

pub struct Opal<'a, 'b> {
    dispatcher: Dispatcher<'a, 'b>,
    world: World,
}

impl<'a, 'b> Opal<'a, 'b> {
    pub fn default_systems() -> DefaultSystems {
        DefaultSystems {
            ai_system: Some(AiSystem::new()),
            map_system: Some(MapSystem::new()),
        }
    }

    pub fn default_dispatcher_start<'c, 'd>(systems: &mut DefaultSystems) -> DispatcherBuilder<'c, 'd> {
        DispatcherBuilder::new()
            .add(systems.ai_system.take().unwrap(), "AiSystem", &[])
    }

    pub fn default_dispatcher_end<'c, 'd>(dispatcher: DispatcherBuilder<'c, 'd>, systems: &mut DefaultSystems) -> DispatcherBuilder<'c, 'd> {
        dispatcher
            .add_barrier()
            .add(systems.map_system.take().unwrap(), "MapSystem", &[])
    }

    pub fn default_world(systems: &DefaultSystems) -> World {
        let mut world = World::new();

        world.register::<AiComponent>();
        world.register::<Position>();

        world.add_resource(systems.map_system.as_ref().unwrap().sender());

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
