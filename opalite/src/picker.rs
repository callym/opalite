use std::cmp::{ Ordering, PartialOrd };
use cgmath::{ prelude::*, Vector3, Vector4 };
use winit::{ ElementState, MouseButton };
use specs::{ Entities, Fetch, System, ReadStorage };
use crate::{
    Camera,
    CollisionLayers,
    InputEvent,
    Map,
    MessageQueue,
    MessageSender,
    MessageReceiver,
    Position,
    RLock,
    Shard,
};

pub struct PickerSystem {
    receiver: MessageReceiver<InputEvent>,
    sender: MessageSender<InputEvent>,
    width: u32,
    height: u32,
}

impl PickerSystem {
    pub fn new(width: u32, height: u32) -> Self {
        let (sender, receiver) = MessageQueue::new();

        Self { sender, receiver, width, height }
    }
}

impl<'a> Shard<'a> for PickerSystem {
    type Message = InputEvent;

    fn sender(&self) -> MessageSender<Self::Message> {
        self.sender.clone()
    }
}

impl<'a> System<'a> for PickerSystem {
    type SystemData = (Fetch<'a, Camera>, Entities<'a>, ReadStorage<'a, Position>, Fetch<'a, RLock<Map>>, ReadStorage<'a, CollisionLayers>);

    fn run(&mut self, (camera, entities, positions, map, _collision_layers): Self::SystemData) {
        use specs::Join;

        let map = map.read().unwrap();

        for message in self.receiver.messages() {
            let (state, button, x, y) = match message {
                InputEvent::MouseClickedWithCoordinates {
                    state, button, x, y
                } => (state, button, x, y),
                _ => continue,
            };

            if state != ElementState::Pressed {
                continue;
            }

            if button != MouseButton::Left {
                continue;
            }

            let x = ((x as f32 * 2.0) / (self.width as f32)) - 1.0;
            let y = ((y as f32 * 2.0) / (self.height as f32)) - 1.0;

            let z = 1.0;
            let w = 1.0;
            let ray_clip = Vector4::new(x, y, z, w);

            let proj_i = camera.projection(self.width as f32 / self.height as f32).invert().unwrap();
            let ray_eye = proj_i * ray_clip;
            let ray_eye = Vector4::new(ray_eye.x, ray_eye.y, -1.0, 0.0);

            let view_i = camera.view().invert().unwrap();
            let ray_world = (view_i * ray_eye).xyz();

            let ray_world = ray_world.normalize();
            let ray_origin = camera.position;

            let mut intersections = vec![];
            for (entity, _) in (&*entities, &positions).join() {
                let position = match map.location(&entity) {
                    Some(position) => Vector3::new(position.x as f32, position.y as f32, position.z as f32),
                    None => continue,
                };

                let l = ray_origin - position;
                let a = ray_world.dot(ray_world);
                let b = 2.0 * ray_world.dot(l);
                let c = l.dot(l) - 1.0;

                if solve_quadratic_roots(a, b, c) == 0 {
                    continue;
                }

                let (root, _) = solve_quadratic(a, b, c);
                let root = root.unwrap();
                intersections.push((root, entity));

                println!("{:?}", position);
            }

            intersections.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(Ordering::Greater));
            println!("{:?}", intersections);
        }
    }
}

fn solve_quadratic_roots(a: f32, b: f32, c: f32) -> u32 {
    let roots = solve_quadratic(a, b, c);

    match roots {
        (Some(_), Some(_)) => 2,
        (Some(_), None) => 1,
        (None, Some(_)) => 1,
        (None, None) => 0,
    }
}

fn solve_quadratic(a: f32, b: f32, c: f32) -> (Option<f32>, Option<f32>) {
    let discr = b.powi(2) - 4.0 * a * c;

    if discr < 0.0 {
        (None, None)
    } else if relative_eq!(discr, 0.0) {
        (Some(-0.5 * b / a), None)
    } else {
        let q = if b > 0.0 {
            -0.5 * (b + discr.sqrt())
        } else {
            -0.5 * (b - discr.sqrt())
        };

        let r_0 = q / a;
        let r_1 = c / q;

        if r_0 < r_1 {
            (Some(r_0), Some(r_1))
        } else {
            (Some(r_1), Some(r_0))
        }
    }
}
