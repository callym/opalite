use crate::{
    AiSystem,
    Config,
    InputEvent,
    MapMessage,
    MessageSender,
    Map,
    MapSystem,
    PickerSystem,
    RLock,
    Shard,
};
use crate::gluon_api::{ GluonUiSystem, DataReferenceSystem, RequireMapSystem };

pub struct DefaultSystems {
    pub(super) ai_system: Option<AiSystem>,
    pub(super) data_ref_system: Option<DataReferenceSystem>,
    pub(super) gluon_ui_system: Option<GluonUiSystem>,
    pub(super) map_system: Option<MapSystem>,
    pub(super) map_system_sender: Option<MessageSender<MapMessage>>,
    pub(super) map_reader: Option<RLock<Map>>,
    pub(super) picker_system: Option<PickerSystem>,
    pub(super) picker_system_sender: Option<MessageSender<InputEvent>>,
    pub(super) require_map_system: Option<RequireMapSystem>,
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
            data_ref_system: Some(DataReferenceSystem::new()),
            gluon_ui_system: Some(GluonUiSystem::new()),
            map_system: Some(map_system),
            map_system_sender: Some(map_system_sender),
            map_reader: Some(map_reader),
            picker_system: Some(picker_system),
            picker_system_sender: Some(picker_system_sender),
            require_map_system: Some(RequireMapSystem::new()),
        }
    }
}
