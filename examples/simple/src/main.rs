#![feature(nll)]

extern crate opalite;

use std::collections::VecDeque;

use opalite::{
    AiComponent,
    AiGoal,
    EventsLoop,
    ModelKey,
    ModelType,
    Opal,
    OpalBuilder,
    Position,
    WindowBuilder,
};

fn main() {
    let mut opal = OpalBuilder::new()
        .add_dispatcher_start()
        .add_dispatcher_end()
        .add_dispatcher_thread_local()
        .add_world()
        .build();

    opal.world_mut().create_entity()
        .with(Position { x: 0, y: 0, z: 0 })
        .with(AiComponent::new(
            Box::new(|goal| {
                match goal {
                    &Some(ref goal) => {
                        match goal {
                            &AiGoal::Move { start, target, .. } => vec![AiGoal::Move {
                                start: target,
                                target: start,
                                path: VecDeque::new(),
                            }],
                            _ => vec![]
                        }
                    },
                    &None => vec![
                        AiGoal::Move {
                            start: Position::new(0, 0, 0),
                            target: Position::new(15, 15, 0),
                            path: VecDeque::new(),
                        }
                    ]
                }
            }),
            Box::new(|_| panic!("AI Error"))))
        .with(ModelKey::new(ModelType::Quad));

    let _ = opal.run();
}
