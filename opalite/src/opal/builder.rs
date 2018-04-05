use std::collections::HashMap;
use cgmath::{ Deg, Vector3 };
use conrod::{ UiBuilder };
use gluon;
use rusttype;
use specs::{ DispatcherBuilder, World };
use winit::{ EventsLoop, WindowBuilder, Window };
use super::{ DefaultSystems, Gluon, GluonUi, Opal, OpalUi, WindowClosed };
use crate::{
    AiComponent,
    Camera,
    CollisionLayers,
    Config,
    ConfigBuilder,
    Data,
    InitialPosition,
    InputEventHandler,
    InputEventType,
    ModelData,
    ModelKey,
    Position,
    Renderer,
    RLock,
    Resources,
};
use crate::gluon_api::{ self, DataReference, GluonUiComponent, RequireMap };
use crate::renderer::MaterialDesc;

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
    gluon: gluon::RootedThread,
    resources: RLock<Resources>,
    window: Option<Window>,
    world: Option<World>,
    #[allow(dead_code)]
    state: S,
}

impl OpalBuilder {
    pub fn new<'a, 'b>() -> PartialOpalBuilder<'a, 'b, BuilderState::New> {
        let config = {
            let mut default_config = Config::from_str(include_str!("../../Opalite.ron")).unwrap();

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

        let resources = Resources::from_config(&config).unwrap();
        let resources = RLock::new(resources);

        let gluon = gluon::new_vm();

        PartialOpalBuilder {
            config,
            default_systems,
            dispatcher: None,
            events_loop: EventsLoop::new(),
            gluon,
            resources,
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

    pub fn gluon(&mut self) -> &mut gluon::RootedThread {
        &mut self.gluon
    }
}

impl<'a, 'b> PartialOpalBuilder<'a, 'b, BuilderState::New> {
    pub fn add_dispatcher_start(mut self) -> PartialOpalBuilder<'a, 'b, BuilderState::DispatcherStart> {
        let dispatcher = DispatcherBuilder::new()
            .add(self.default_systems.data_ref_system.take().unwrap(), "DataReferenceSystem", &[])
            .add(self.default_systems.require_map_system.take().unwrap(), "RequireMapSystem", &[]);

        PartialOpalBuilder {
            config: self.config,
            default_systems: self.default_systems,
            dispatcher: Some(dispatcher),
            events_loop: self.events_loop,
            gluon: self.gluon,
            resources: self.resources,
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
            .add(self.default_systems.picker_system.take().unwrap(), "PickerSystem", &[])
            .add(self.default_systems.ai_system.take().unwrap(), "AiSystem", &[])
            .add(self.default_systems.map_system.take().unwrap(), "MapSystem", &["AiSystem"]);

        PartialOpalBuilder {
            config: self.config,
            default_systems: self.default_systems,
            dispatcher: Some(dispatcher),
            events_loop: self.events_loop,
            gluon: self.gluon,
            resources: self.resources,
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

        let renderer = Renderer::new(self.config.clone(), self.resources.clone(), &window).unwrap();

        let dispatcher = self.dispatcher.take()
            .unwrap()
            .add_barrier()
            .add(self.default_systems.gluon_ui_system.take().unwrap(), "GluonUiSystem", &[])
            .add_thread_local(renderer);

        PartialOpalBuilder {
            config: self.config,
            default_systems: self.default_systems,
            dispatcher: Some(dispatcher),
            events_loop: self.events_loop,
            gluon: self.gluon,
            resources: self.resources,
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
            world.register::<Data>();
            world.register::<DataReference>();
            world.register::<GluonUiComponent>();
            world.register::<MaterialDesc>();
            world.register::<ModelData>();
            world.register::<ModelKey>();
            world.register::<InitialPosition>();
            world.register::<Position>();
            world.register::<RequireMap>();

            world.add_resource(self.default_systems.map_reader.take().unwrap());
            world.add_resource(self.default_systems.map_system_sender.take().unwrap());
            world.add_resource(self.config.clone());
            world.add_resource(WindowClosed(false));
            world.add_resource(Camera {
                position: Vector3::new(1.0, 2.5, 5.0),
                direction: Vector3::new(0.0, -0.5, -1.0),
                fovy: Deg(45.0),
                near: 0.1,
                far: 100.0,
            });

            world
        };

        PartialOpalBuilder {
            config: self.config,
            default_systems: self.default_systems,
            dispatcher: self.dispatcher,
            events_loop: self.events_loop,
            gluon: self.gluon,
            resources: self.resources,
            window: self.window,
            world: Some(world),
            state: BuilderState::World,
        }
    }
}

impl<'a, 'b> PartialOpalBuilder<'a, 'b, BuilderState::World> {
    pub fn build(self) -> Opal<'a, 'b> {
        let PartialOpalBuilder { config, mut default_systems, dispatcher, events_loop, gluon, resources, window, world, .. } = self;
        let dispatcher = dispatcher.unwrap().build();
        let window = window.unwrap();
        let mut world = world.unwrap();

        let mut input_event_handler = InputEventHandler::new();
        input_event_handler.register(InputEventType::MouseClickedWithCoordinates, default_systems.picker_system_sender.take().unwrap());

        let (width, height) = window.get_inner_size().unwrap();
        let mut ui = UiBuilder::new([width as f64, height as f64])
            .build();

        for font in &config.fonts {
            let resources = resources.read().unwrap();
            let bytes = resources.get(font).unwrap();
            let font = rusttype::Font::from_bytes(bytes).unwrap();
            ui.fonts.insert(font);
        }

        world.add_resource(OpalUi(None));
        world.add_resource(GluonUi(HashMap::new()));
        world.add_resource(resources);

        gluon_api::register_opalite_api(&gluon);

        world.add_resource(Gluon {
            thread: gluon,
            compiler: gluon::Compiler::new().run_io(true),
        });

        Opal { config, dispatcher, events_loop, input_event_handler, ui, window, world }
    }
}
