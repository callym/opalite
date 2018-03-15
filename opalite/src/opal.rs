use std::cmp::PartialEq;
use specs::{ DispatcherBuilder, Dispatcher, World };
use winit::{ EventsLoop, WindowBuilder, Window };
use crate::{ AiComponent, AiSystem, Config, ConfigBuilder, MapMessage, MessageSender, ModelKey, MapSystem, Position, Renderer, Shard };

pub struct WindowClosed(bool);

impl PartialEq<bool> for WindowClosed {
    fn eq(&self, other: &bool) -> bool {
        &self.0 == other
    }
}

pub struct DefaultSystems {
    ai_system: Option<AiSystem>,
    map_system: Option<MapSystem>,
    map_system_sender: Option<MessageSender<MapMessage>>,
}

impl DefaultSystems {
    pub fn new() -> Self {
        let map_system = MapSystem::new();
        let map_system_sender = map_system.sender();

        Self {
            ai_system: Some(AiSystem::new()),
            map_system: Some(map_system),
            map_system_sender: Some(map_system_sender),
        }
    }
}

pub struct Opal<'a, 'b> {
    config: Config,
    dispatcher: Dispatcher<'a, 'b>,
    events_loop: EventsLoop,
    #[allow(dead_code)]
    window: Window,
    world: World,
}

#[allow(non_snake_case)]
mod BuilderState {
    pub struct New;
    pub struct DispatcherStart;
    pub struct DispatcherEnd;
    pub struct DispatcherThreadLocal;
    pub struct World;
}

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

pub struct OpalBuilder;

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

        PartialOpalBuilder {
            config,
            default_systems: DefaultSystems::new(),
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
            .add_barrier()
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
            world.register::<ModelKey>();
            world.register::<Position>();

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

impl<'a, 'b> Opal<'a, 'b> {
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn run(&mut self) -> Result<(), ()> {
        use winit::{ Event, WindowEvent };

        let Opal { events_loop, dispatcher, world, .. } = self;

        while *world.read_resource::<WindowClosed>() == false {
            events_loop.poll_events(|event| {
                if let Event::WindowEvent { event, .. } = event {
                    match event {
                        WindowEvent::Closed => {
                            let mut window_closed = world.write_resource::<WindowClosed>();
                            *window_closed = WindowClosed(true);
                        },
                        _ => (),
                    }
                }
            });

            dispatcher.dispatch(&mut world.res);

            //::std::thread::sleep(::std::time::Duration::from_millis(250));
        }

        Ok(())
    }
}
