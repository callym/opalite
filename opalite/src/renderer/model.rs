use std::{ path::PathBuf, sync::{ Arc, Mutex } };
use back::Backend as B;
use hal::{ self, Backend };
use uuid::Uuid;
use crate::renderer::Buffer;

#[derive(BufferData, Copy, Clone, Debug)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
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

#[derive(Hash, PartialEq, Eq, Clone)]
pub enum ModelType {
    Quad,
    File(PathBuf),
}

pub struct Model {
    pub vertex_buffer: Buffer<Vertex, B>,
    pub index_buffer: Buffer<u32, B>,
}

impl Model {
    pub(super) fn quad(color: [f32; 3], device: Arc<Mutex<<B as Backend>::Device>>, memory_types: &[hal::MemoryType]) -> Self {
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
}

fn make_quad(color: [f32; 3]) -> [Vertex; 6] {[
  Vertex { position: [ -0.5, 0.5, 0.0 ], color, .. Default::default() },
  Vertex { position: [  0.5, 0.5, 0.0 ], color, .. Default::default() },
  Vertex { position: [  0.5,-0.5, 0.0 ], color, .. Default::default() },

  Vertex { position: [ -0.5, 0.5, 0.0 ], color, .. Default::default() },
  Vertex { position: [  0.5,-0.5, 0.0 ], color, .. Default::default() },
  Vertex { position: [ -0.5,-0.5, 0.0 ], color, .. Default::default() },
]}
