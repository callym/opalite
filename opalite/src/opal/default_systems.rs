use std::cmp::PartialEq;
use cgmath::Vector3;
use specs::{ DispatcherBuilder, Dispatcher, World };
use winit::{ EventsLoop, WindowBuilder, Window };
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

pub struct DefaultSystems {
    pub(super) ai_system: Option<AiSystem>,
    pub(super) map_system: Option<MapSystem>,
    pub(super) map_system_sender: Option<MessageSender<MapMessage>>,
    pub(super) map_reader: Option<RLock<Map>>,
}

impl DefaultSystems {
    pub fn new(config: &Config) -> Self {
        let map_dimensions = config.map_dimensions.into();
        let map_system = MapSystem::new(map_dimensions);
        let map_system_sender = map_system.sender();
        let map_reader = map_system.map();

        Self {
            ai_system: Some(AiSystem::new()),
            map_system: Some(map_system),
            map_system_sender: Some(map_system_sender),
            map_reader: Some(map_reader),
        }
    }
}
