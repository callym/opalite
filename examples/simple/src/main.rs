#![feature(nll)]

extern crate opalite;

use std::collections::VecDeque;

use opalite::cgmath::Vector3;
use opalite::{
    AiComponent,
    AiGoalDo,
    AiGoal,
    CollisionLayer,
    CollisionLayers,
    ModelKey,
    ModelType,
    OpalBuilder,
    InitialPosition,
};

fn main() {
    let mut opal = OpalBuilder::new()
        .add_dispatcher_start()
        .add_dispatcher_end()
        .add_dispatcher_thread_local()
        .add_world()
        .build();

    opal.world_mut().create_entity()
        .with(InitialPosition((0, 0, 0).into()))
        .with(CollisionLayers::new([CollisionLayer::PLAYER].iter()))
        .with(AiComponent::new(
            Box::new(|goal| AiGoalDo::Continue),
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
                            start: Vector3::new(0, 0, 0),
                            target: Vector3::new(15, 15, 0),
                            path: VecDeque::new(),
                        }
                    ]
                }
            }),
            Box::new(|_| panic!("AI Error"))))
        .with(ModelKey::new(ModelType::Hex));

    let _ = opal.run();
}
