use std::cmp::PartialEq;
use cgmath::Vector3;
use specs::{ DispatcherBuilder, Dispatcher, World };
use winit::{ EventsLoop, WindowBuilder, Window };
use super::{ DefaultSystems, Opal, WindowClosed };
use crate::{
    AiComponent,
    AiSystem,
    CollisionLayers,
    Config,
    ConfigBuilder,
    InitialPosition,
    MapMessage,
    MessageSender,
    ModelData,
    ModelKey,
    Map,
    MapSystem,
    Position,
    Renderer,
    RLock,
    Shard,
};

#[allow(non_snake_case)]
mod BuilderState {
    pub struct New;
    pub struct DispatcherStart;
    pub struct DispatcherEnd;
    pub struct DispatcherThreadLocal;
    pub struct World;
}

pub struct OpalBuilder;

pub struct PartialOpalBuilder<'a, 'b, S> {
    config: Config,
    default_systems: DefaultSystems,
    dispatcher: Option<DispatcherBuilder<'a, 'b>>,
    events_loop: EventsLoop,
    window: Option<Window>,
    world: Option<World>,
    #[allow(dead_code)]
    state: S,
}

impl OpalBuilder {
    pub fn new<'a, 'b>() -> PartialOpalBuilder<'a, 'b, BuilderState::New> {
        let config = {
            let mut default_config = Config::from_file(format!("{}/Opalite.ron", env!("CARGO_MANIFEST_DIR"))).unwrap();
            let cwd = {
                let mut cwd = ::std::env::current_dir().unwrap();
                cwd.push("Opalite.ron");
                cwd
            };

            match ConfigBuilder::from_file(cwd) {
                Ok(config) => default_config.merge(config),
                Err(_) => default_config,
            }
        };

        let default_systems = DefaultSystems::new(&config);

        PartialOpalBuilder {
            config,
            default_systems,
            dispatcher: None,
            events_loop: EventsLoop::new(),
            window: None,
            world: None,
            state: BuilderState::New,
        }
    }
}

impl<'a, 'b, S> PartialOpalBuilder<'a, 'b, S> {
    pub fn dispatcher_builder(&mut self) -> Option<&mut DispatcherBuilder<'a, 'b>> {
        self.dispatcher.as_mut()
    }
}

impl<'a, 'b> PartialOpalBuilder<'a, 'b, BuilderState::New> {
    pub fn add_dispatcher_start(mut self) -> PartialOpalBuilder<'a, 'b, BuilderState::DispatcherStart> {
        let dispatcher = DispatcherBuilder::new()
            .add(self.default_systems.ai_system.take().unwrap(), "AiSystem", &[]);

        PartialOpalBuilder {
            config: self.config,
            default_systems: self.default_systems,
            dispatcher: Some(dispatcher),
            events_loop: self.events_loop,
            window: None,
            world: self.world,
            state: BuilderState::DispatcherStart,
        }
    }
}

impl<'a, 'b> PartialOpalBuilder<'a, 'b, BuilderState::DispatcherStart> {
    pub fn add_dispatcher_end(mut self) -> PartialOpalBuilder<'a, 'b, BuilderState::DispatcherEnd> {
        let dispatcher = self.dispatcher.take()
            .unwrap()
            .add_barrier()
            .add(self.default_systems.map_system.take().unwrap(), "MapSystem", &[]);

        PartialOpalBuilder {
            config: self.config,
            default_systems: self.default_systems,
            dispatcher: Some(dispatcher),
            events_loop: self.events_loop,
            window: None,
            world: self.world,
            state: BuilderState::DispatcherEnd,
        }
    }
}

impl<'a, 'b> PartialOpalBuilder<'a, 'b, BuilderState::DispatcherEnd> {
    pub fn add_dispatcher_thread_local(mut self) -> PartialOpalBuilder<'a, 'b, BuilderState::DispatcherThreadLocal> {
        let window = WindowBuilder::new()
            .with_dimensions(self.config.window_dimensions.0, self.config.window_dimensions.1)
            .with_title(self.config.title.clone())
            .build(&self.events_loop)
            .unwrap();

        let renderer = Renderer::new(self.config.clone(), &window).unwrap();

        let dispatcher = self.dispatcher.take()
            .unwrap()
            .add_thread_local(renderer);

        PartialOpalBuilder {
            config: self.config,
            default_systems: self.default_systems,
            dispatcher: Some(dispatcher),
            events_loop: self.events_loop,
            window: Some(window),
            world: self.world,
            state: BuilderState::DispatcherThreadLocal,
        }
    }
}

impl<'a, 'b> PartialOpalBuilder<'a, 'b, BuilderState::DispatcherThreadLocal> {
    pub fn add_world(mut self) -> PartialOpalBuilder<'a, 'b, BuilderState::World> {
        let world = {
            let mut world = World::new();

            world.register::<AiComponent>();
            world.register::<CollisionLayers>();
            world.register::<ModelData>();
            world.register::<ModelKey>();
            world.register::<InitialPosition>();
            world.register::<Position>();

            world.add_resource(self.default_systems.map_reader.take().unwrap());
            world.add_resource(self.default_systems.map_system_sender.take().unwrap());
            world.add_resource(self.config.clone());
            world.add_resource(WindowClosed(false));

            world
        };

        PartialOpalBuilder {
            config: self.config,
            default_systems: self.default_systems,
            dispatcher: self.dispatcher,
            events_loop: self.events_loop,
            window: self.window,
            world: Some(world),
            state: BuilderState::World,
        }
    }
}

impl<'a, 'b> PartialOpalBuilder<'a, 'b, BuilderState::World> {
    pub fn build(self) -> Opal<'a, 'b> {
        let PartialOpalBuilder { config, dispatcher, events_loop, window, world, .. } = self;
        let dispatcher = dispatcher.unwrap().build();
        let window = window.unwrap();
        let world = world.unwrap();

        Opal { config, dispatcher, events_loop, window, world }
    }
}