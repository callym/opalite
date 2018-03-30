use std::{ collections::{ HashMap, HashSet }, ops, sync::{ mpsc, Arc } };
use cgmath::Vector3;
use specs::{ Entities, Entity, System, ReadStorage, VecStorage, WriteStorage };
use crate::{
    Message,
    MessageQueue,
    MessageSender,
    MessageReceiver,
    Shard,
    RLock,
    WLock,
};

#[derive(Component, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InitialPosition(pub Vector3<i32>);

impl InitialPosition {
    pub fn x(&self) -> f64 {
        self.0.x as f64
    }

    pub fn y(&self) -> f64 {
        self.0.y as f64
    }

    pub fn z(&self) -> f64 {
        self.0.z as f64
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CollisionLayer(pub i32);

impl CollisionLayer {
    pub const PLAYER: Self = CollisionLayer(1);
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct CollisionLayers(pub HashSet<CollisionLayer>);

impl CollisionLayers {
    pub fn new<'a>(layers: impl Iterator<Item = &'a CollisionLayer>) -> Self {
        CollisionLayers(layers.map(|l| *l).collect())
    }
}

impl ops::Deref for CollisionLayers {
    type Target = HashSet<CollisionLayer>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Vector3<i32>> for InitialPosition {
    fn from(vec: Vector3<i32>) -> InitialPosition {
        let vec = vec.into();
        InitialPosition(vec)
    }
}

#[derive(Component, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Position;

#[derive(Clone)]
pub enum MapMessage {
    Move { entity: Entity, new_location: Vector3<i32>, absolute: bool, reply: Option<mpsc::SyncSender<bool>> },
}

impl Message for MapMessage { }

#[derive(Debug, Clone)]
pub struct Map {
    width: i32,
    height: i32,
    depth: i32,
    cells: HashMap<Vector3<i32>, HashSet<Entity>>,
    entities: HashMap<Entity, Vector3<i32>>,
}

impl Map {
    pub fn new(width: i32, depth: i32, height: i32) -> Self {
        Self {
            width,
            depth,
            height,
            cells: HashMap::new(),
            entities: HashMap::new(),
        }
    }

    pub fn location(&self, entity: &Entity) -> Option<&Vector3<i32>> {
        self.entities.get(entity)
    }

    pub fn entities(&self, location: &Vector3<i32>) -> Option<impl Iterator<Item = &Entity>> {
        if let Some(entities) = self.cells.get(location) {
            if entities.len() == 0 {
                None
            } else {
                Some(entities.iter())
            }
        } else {
            None
        }
    }

    pub fn can_move<'a>(&self, entity: Entity, location: Vector3<i32>, collision_layers: &ReadStorage<'a, CollisionLayers>) -> bool {
        let collisions_to_check = match collision_layers.get(entity) {
            Some(layers) => layers,
            // if there aren't any collision layers then you can move
            None => return true,
        };

        if let Some(entities) = self.entities(&location) {
            for entity in entities {
                if let Some(collisions) = collision_layers.get(*entity) {
                    for collision in collisions.iter() {
                        if collisions_to_check.contains(collision) {
                            return false;
                        }
                    }
                }
            }
        }

        true
    }

    pub fn move_entity(&mut self, entity: Entity, location: Vector3<i32>) {
        if let Some(previous_location) = self.entities.get(&entity) {
            let previous_location = self.cells.get_mut(previous_location).unwrap();
            previous_location.remove(&entity);
        }

        self.entities.insert(entity, location);
        self.cells.entry(location)
            .or_insert_with(|| {
                let mut entities = HashSet::new();
                entities.insert(entity);
                entities
            });
    }
}

pub struct MapSystem {
    receiver: MessageReceiver<MapMessage>,
    sender: MessageSender<MapMessage>,
    map: WLock<Map>,
}

impl MapSystem {
    pub fn new(map_size: Vector3<i32>) -> Self {
        let (sender, receiver) = MessageQueue::new();

        let Vector3 { x: width, y: depth, z: height } = map_size;

        let map = WLock::new(Map::new(width, depth, height));

        Self { sender, receiver, map }
    }

    pub fn map(&self) -> RLock<Map> {
        self.map.get_reader()
    }
}

impl<'a> Shard<'a> for MapSystem {
    type Message = MapMessage;

    fn sender(&self) -> MessageSender<Self::Message> {
        self.sender.clone()
    }
}

impl<'a> System<'a> for MapSystem {
    type SystemData = (Entities<'a>, ReadStorage<'a, InitialPosition>, ReadStorage<'a, CollisionLayers>, WriteStorage<'a, Position>);

    fn run(&mut self, (entities, initial_positions, collision_layers, mut positions): Self::SystemData) {
        use specs::Join;

        let mut map = self.map.write().unwrap();

        let mut entities_to_add_position = vec![];
        for (entity, initial_position, _) in (&*entities, &initial_positions, !&positions).join() {
            // TODO - move to closest valid tile?
            map.move_entity(entity, initial_position.0);
            entities_to_add_position.push(entity);
        }

        for entity in entities_to_add_position {
            positions.insert(entity, Position);
        }

        for message in self.receiver.messages() {
            use self::MapMessage::*;
            match message {
                Move { entity, new_location, absolute, reply } => {
                    let location = map.location(&entity)
                        .map(|v| *v)
                        .unwrap_or(Vector3::new(0, 0, 0));

                    let new_location = if absolute { new_location } else { location + new_location };

                    if  new_location.x < 0 || new_location.x > map.width ||
                        new_location.y < 0 || new_location.y > map.height ||
                        new_location.z < 0 || new_location.z > map.depth {
                            reply.map(|reply| reply.send(false).unwrap());
                            continue;
                    }

                    if map.can_move(entity, new_location, &collision_layers) {
                        map.move_entity(entity, new_location);
                        reply.map(|reply| reply.send(true).unwrap());
                    } else {
                        reply.map(|reply| reply.send(false).unwrap());
                    }
                },
            }
        }
    }
}
