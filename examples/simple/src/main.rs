#![feature(match_default_bindings, nll)]
use std::env;

extern crate opalite;
#[macro_use] extern crate log;
extern crate env_logger;

use std::{
    collections::VecDeque,
    path::PathBuf,
};

use opalite::cgmath::Vector3;
use opalite::{
    AiComponent,
    AiGoalDo,
    AiGoal,
    CollisionLayer,
    CollisionLayers,
    Data,
    Light,
    LightType,
    MaterialDesc,
    ModelData,
    ModelKey,
    ModelType,
    OpalBuilder,
    InitialPosition,
    SurfaceType,
};
use opalite::renderer::conv::*;
use opalite::gluon_api::DataReference;
use opalite::gluon_api::GluonUiComponent;
use opalite::gluon_api::RequireMap;

mod map;

fn main() {
    env_logger::init();
    info!("starting up");
    env::set_current_dir("examples/simple").unwrap();

    let mut opal = OpalBuilder::new()
        .add_dispatcher_start()
        .add_dispatcher_end()
        .add_dispatcher_thread_local()
        .add_world()
        .build();

    /*opal.world_mut().create_entity()
        .with(InitialPosition((0, 1, 0).into()))
        .with(CollisionLayers::new([CollisionLayer::PLAYER].iter()))
        .with(ModelKey::new(ModelType::Hex))
        .with(MaterialDesc {
            diffuse: SurfaceType::Color(vec4(0.75, 0.0, 0.0, 1.0)),
            specular: float(10.0),
        })
        .build();*/

    opal.world_mut().create_entity()
        .with(InitialPosition((0, 1, 0).into()))
        .with(CollisionLayers::new([CollisionLayer::PLAYER].iter()))
        .with(ModelKey::new(ModelType::File("Suzanne/glTF/Suzanne.glb".into())))
        .with(MaterialDesc {
            diffuse: SurfaceType::Color(vec4(1.0, 1.0, 1.0, 1.0)),
            specular: float(10.0),
        })
        .with(ModelData {
            scale: Vector3::new(0.5, 0.5, 0.5),
            .. Default::default()
        })
        .build();

    let entity = opal.world_mut().create_entity()
        .with(InitialPosition((0, 0, 0).into()))
        .with(CollisionLayers::new([CollisionLayer::PLAYER].iter()))
        .with(AiComponent::new(
            Box::new(|_| AiGoalDo::Continue),
            Box::new(|goal| {
                match goal {
                    Some(ref goal) => {
                        match goal {
                            &AiGoal::Move { start, target, .. } => vec![AiGoal::Move {
                                start: target,
                                target: start,
                                path: VecDeque::new(),
                            }],
                        }
                    },
                    None => vec![
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
        .with(MaterialDesc {
            diffuse: SurfaceType::Color(vec4(0.5, 0.5, 0.0, 1.0)),
            specular: float(32.0),
        })
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
let { oval, rgba } = import! conrod

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
    oval.new 0.5 0.5 \"rect\" |>
        oval.color color |>
        oval.x_y 0.5 0.5 |>
        oval.build in rect
            "),
        })
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
            name: String::from("Testing Ui - Text"),
            expr: String::from("
let prelude = import! std.prelude
let string = import! std.string
let { (|>) } = import! std.function
let { unwrap } = import! std.option
let { (<>) } = prelude.make_Semigroup string.semigroup

let data_reference = import! data_reference
let initial_position = import! initial_position
let map_ref = import! map
let { vec3 } = import! cgmath
let { text, rgba } = import! conrod

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

let rect =
    text.new (\"hello world!\" <> (vec3.to_string position)) \"text-test\"
        |> text.color (rgba 0.0 0.0 0.0 1.0)
        |> text.x_y 0.5 0.5
        |> text.font_size 32
        |> text.build in rect
            "),
        })
        .build();

    opal.world_mut().create_entity()
        .with(InitialPosition((1, 1, 0).into()))
//        .with(ModelKey::new(ModelType::Sphere))
        .with(ModelData {
            translate: Vector3::new(0.0, 0.0, 1.0),
//            scale: [0.3; 3].into(),
            .. Default::default()
        })
        .with(MaterialDesc {
            diffuse: SurfaceType::Color(vec4(1.0, 0.0, 1.0, 1.0)),
            specular: float(32.0),
        })
        .with(Light {
            ty: LightType::Point,
            color: Vector3::new(0.5, 0.7, 0.8),
        })
        .build();

    map::HexGrid::new(15, 15, opal.world_mut());

    let _ = opal.run();
}
