use std::path::PathBuf;
use uuid::Uuid;

#[derive(BufferData, Copy, Clone, Debug)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
    // pads to 8
    _padding: [f32; 2],
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

}
