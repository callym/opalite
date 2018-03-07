use std::{ collections::HashMap, ops, sync::mpsc };
use specs::{ Entity, ReadStorage, System, VecStorage, WriteStorage };
use crate::MessageQueue;

#[derive(Component, Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[component(VecStorage)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Position {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

impl ops::Add for Position {
    type Output = Position;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl ops::AddAssign for Position {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

#[derive(Clone)]
pub enum MapMessage {
    Move { entity: Entity, position: Position, absolute: bool, reply: Option<mpsc::SyncSender<bool>> },
}

pub struct MapSystem {
    messages: mpsc::Receiver<MapMessage>,
    sender: MessageQueue<MapMessage>,
}

impl MapSystem {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let sender = MessageQueue::new(sender);

        Self {
            messages: receiver,
            sender: sender,
        }
    }

    pub fn sender(&self) -> MessageQueue<MapMessage> {
        self.sender.clone()
    }
}

impl<'a> System<'a> for MapSystem {
    type SystemData = WriteStorage<'a, Position>;

    fn run(&mut self, mut positions: Self::SystemData) {
        use specs::Join;

        let mut blocked = HashMap::new();

        for position in positions.join() {
            blocked.insert(*position, true);
        }

        for message in self.messages.try_iter() {
            use self::MapMessage::*;
            match message {
                Move { entity, position, absolute, reply } => {
                    let position = if absolute { position } else { *positions.get(entity).unwrap() + position };

                    if blocked.contains_key(&position) {
                        reply.map(|reply| reply.send(false).unwrap());
                    } else {
                        positions.insert(entity, position);
                        reply.map(|reply| reply.send(true).unwrap());
                    }
                },
            }
        }
    }
}
