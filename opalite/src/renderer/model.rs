use std::{ self, path::PathBuf, sync::{ Arc, Mutex } };
use back::Backend as B;
use hal::{ self, Backend };
use cgmath::{ prelude::*, Matrix4, Vector3 };
use uuid::Uuid;
use crate::renderer::Buffer;

#[derive(BufferData, Copy, Clone, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    pub fn x(&self) -> f32 {
        self.position[0]
    }

    pub fn y(&self) -> f32 {
        self.position[1]
    }

    pub fn z(&self) -> f32 {
        self.position[2]
    }

    pub fn set_x(&mut self, val: f32) {
        self.position[0] = val;
    }

    pub fn set_y(&mut self, val: f32) {
        self.position[1] = val;
    }

    pub fn set_z(&mut self, val: f32) {
        self.position[2] = val;
    }
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            color: [1.0, 1.0, 0.0],
        }
    }
}

#[derive(Component, Hash, PartialEq, Eq, Clone)]
pub struct ModelKey(ModelType, Uuid);

impl ModelKey {
    pub fn new(model_type: ModelType) -> Self {
        ModelKey(model_type, Uuid::new_v4())
    }

    pub fn ty(&self) -> &ModelType {
        &self.0
    }
}

#[derive(Component, PartialEq, Clone, Copy)]
pub struct ModelData {
    pub ignore_position: bool,
    pub translate: Vector3<f32>,
    pub scale: Vector3<f32>,
}

impl Default for ModelData {
    fn default() -> ModelData {
        ModelData {
            ignore_position: false,
            translate: Vector3::new(0.0, 0.0, 0.0),
            scale: Vector3::new(0.01, 0.01, 0.01),
        }
    }
}

impl ModelData {
    pub fn to_matrix(&self, position: &Vector3<i32>) -> Matrix4<f32> {
        let position = if self.ignore_position {
            Vector3::new(0.0, 0.0, 0.0)
        } else {
            Vector3::new(position.x as f32, position.y as f32, position.z as f32)
        };

        let translate = self.translate + position;
        let scale = Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.x);
        let translate = Matrix4::from_translation(translate);

        (scale * translate).into()
    }
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub enum ModelType {
    Quad,
    Hex,
    File(PathBuf),
}

pub struct Model {
    pub vertex_buffer: Buffer<Vertex, B>,
    pub index_buffer: Buffer<u32, B>,
}

impl Model {
    pub(crate) fn quad(color: [f32; 3], device: Arc<Mutex<<B as Backend>::Device>>, memory_types: &[hal::MemoryType]) -> Self {
        let vertices = make_quad(color).to_vec();
        let mut vertex_buffer = Buffer::<Vertex, B>::new(device.clone(), vertices.len() as u64, hal::buffer::Usage::VERTEX, &memory_types).unwrap();
        vertex_buffer.write(&vertices[..]).unwrap();

        let indices = (0..6 as u32).collect::<Vec<_>>();
        let mut index_buffer = Buffer::<u32, B>::new(device.clone(), indices.len() as u64, hal::buffer::Usage::INDEX, &memory_types).unwrap();
        index_buffer.write(&indices[..]).unwrap();

        Self {
            vertex_buffer,
            index_buffer,
        }
    }

    pub(crate) fn hex(color: [f32; 3], device: Arc<Mutex<<B as Backend>::Device>>, memory_types: &[hal::MemoryType]) -> Self {
        let (vertices, indices) = make_hex(color);

        let mut vertex_buffer = Buffer::<Vertex, B>::new(device.clone(), vertices.len() as u64, hal::buffer::Usage::VERTEX, &memory_types).unwrap();
        vertex_buffer.write(&vertices[..]).unwrap();

        let mut index_buffer = Buffer::<u32, B>::new(device.clone(), indices.len() as u64, hal::buffer::Usage::INDEX, &memory_types).unwrap();
        index_buffer.write(&indices[..]).unwrap();

        Self {
            vertex_buffer,
            index_buffer,
        }
    }
}

fn make_quad(color: [f32; 3]) -> [Vertex; 6] {[
  Vertex { position: [ -0.5, 0.5, 0.0 ], color, .. Default::default() },
  Vertex { position: [  0.5, 0.5, 0.0 ], color, .. Default::default() },
  Vertex { position: [  0.5,-0.5, 0.0 ], color, .. Default::default() },

  Vertex { position: [ -0.5, 0.5, 0.0 ], color, .. Default::default() },
  Vertex { position: [  0.5,-0.5, 0.0 ], color, .. Default::default() },
  Vertex { position: [ -0.5,-0.5, 0.0 ], color, .. Default::default() },
]}

fn make_hex(color: [f32; 3]) -> ([Vertex; 7], [u32; 18]) {
    let start_angle = 0.5;

    let offset = |corner| {
        let angle: f32 = 2.0 * std::f32::consts::PI * (start_angle + corner) / 6.0;
        (angle.cos(), angle.sin())
    };

    let mut arr: [Vertex; 7] = Default::default();
    for (i, v) in arr.iter_mut().enumerate() {
        v.color = color;

        if i == 0 {
            v.set_x(0.0);
            v.set_y(0.0);
            continue;
        }

        let (x, y) = offset(i as f32);
        v.set_x(x);
        v.set_y(y);
    }

    let indices = [
        0, 1, 2,
        0, 2, 3,
        0, 3, 4,
        0, 4, 5,
        0, 5, 6,
        0, 6, 1,
    ];

    (arr, indices)
}
