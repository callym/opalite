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
    InputEvent,
    MapMessage,
    MessageSender,
    ModelData,
    ModelKey,
    Map,
    MapSystem,
    PickerSystem,
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
    pub(super) picker_system: Option<PickerSystem>,
    pub(super) picker_system_sender: Option<MessageSender<InputEvent>>,
}

impl DefaultSystems {
    pub fn new(config: &Config) -> Self {
        let map_dimensions = config.map_dimensions.into();
        let map_system = MapSystem::new(map_dimensions);
        let map_system_sender = map_system.sender();
        let map_reader = map_system.map();
        let (width, height) = config.window_dimensions;
        let picker_system = PickerSystem::new(width, height);
        let picker_system_sender = picker_system.sender();

        Self {
            ai_system: Some(AiSystem::new()),
            map_system: Some(map_system),
            map_system_sender: Some(map_system_sender),
            map_reader: Some(map_reader),
            picker_system: Some(picker_system),
            picker_system_sender: Some(picker_system_sender),
        }
    }
}
