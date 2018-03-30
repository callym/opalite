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
    Data,
    ModelKey,
    ModelType,
    OpalBuilder,
    InitialPosition,
};
use opalite::gluon_api::DataReference;
use opalite::gluon_api::GluonUiComponent;
use opalite::gluon_api::RequireMap;

mod map;

fn main() {
    let mut opal = OpalBuilder::new()
        .add_dispatcher_start()
        .add_dispatcher_end()
        .add_dispatcher_thread_local()
        .add_world()
        .build();

    opal.world_mut().create_entity()
        .with(InitialPosition((0, 1, 0).into()))
        .with(CollisionLayers::new([CollisionLayer::PLAYER].iter()))
        .with(ModelKey::new(ModelType::Hex))
        .build();

    let entity = opal.world_mut().create_entity()
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
                            target: Vector3::new(15, 0, 15),
                            path: VecDeque::new(),
                        }
                    ]
                }
            }),
            Box::new(|_| panic!("AI Error"))))
        .with(ModelKey::new(ModelType::Sphere))
        .build();

    let data = Data::new();
    data.insert(InitialPosition((0, 10, 10).into()));
    data.insert(DataReference {
        entity: Some(entity),
        map: None,
    });

    opal.world_mut().create_entity()
        .with(RequireMap)
        .with(data)
        .with(GluonUiComponent {
            name: String::from("Testing Ui"),
            expr: String::from("
let { (|>) } = import! std.function
let { unwrap } = import! std.option
let data_reference = import! data_reference
let initial_position = import! initial_position
let map_ref = import! map
let { vec3 } = import! cgmath
let { bordered_rectangle, rgba } = import! conrod

let map = unwrap (map_ref.data.get data)

let position =
    match data_reference.get data with
    | Some ref ->
        match data_reference.entity ref with
            | Some entity ->
                match map_ref.location map entity with
                    | Some location -> location
                    | None -> vec3.new 0.0 0.0 0.0
            | None -> vec3.new 0.0 0.0 0.0
    | None -> vec3.new 0.0 0.0 0.0

let color = rgba (vec3.x position / 10.0) (vec3.y position / 10.0) (vec3.z position / 10.0) 1.0

let rect =
    bordered_rectangle.new 0.5 0.5 \"rect\" |>
        bordered_rectangle.color color |>
        bordered_rectangle.x_y 0.5 0.5 |>
        bordered_rectangle.border 0.55 |>
        bordered_rectangle.border_color (rgba 0.0 0.0 0.0 1.0) |>
        bordered_rectangle.build in rect
            "),
        })
        .build();

    map::HexGrid::new(15, 15, opal.world_mut());

    let _ = opal.run();
}

/*
let position =
    match data_reference.get data with
    | Some ref ->
        match ref.entity with
            | Some entity -> location map entity
            | None -> vec3.new 0.0 0.0 0.0
    | None -> vec3.new 0.0 0.0 0.0

let color = rgba (position.x / 10.0) (position.y / 10.0) (position.z / 10.0) 1.0
*/
