#![feature(nll)]

extern crate opalite;

use std::collections::VecDeque;

use opalite::{
    AiComponent,
    AiGoal,
    Opal,
    Position,
};

fn main() {
    let mut opal = {
        let systems = Opal::default_systems();
        let world = Opal::default_world(&systems);
        let dispatcher = Opal::default_dispatcher(systems);

        Opal::new(world, dispatcher)
    };

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
            Box::new(|_| {
                panic!()
            }))
        );

    loop {
        let _ = opal.run();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
