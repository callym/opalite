use std::path::PathBuf;
use gfx::{ self, handle::Buffer, Device };
use back::Backend as B;
use hal;
use uuid::Uuid;

use crate::renderer::Allocator;

gfx_buffer_struct! {
    Vertex {
        position: [f32; 2],
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
    pub vertex_buffer: Buffer<B, Vertex>,
    pub vertex_count: u32,
    pub index_buffer: Buffer<B, u32>,
    pub index_count: u32,
}

impl Model {
    pub(super) fn quad(device: &mut Device<B>, encoder: &mut gfx::Encoder<B, hal::Graphics>, upload: &mut Allocator) -> Model {
        let vertices = QUAD.to_vec();
        let vertex_count = QUAD.len() as u32;
        let indices = (0..QUAD.len() as u32).collect::<Vec<_>>();
        let index_count = indices.len() as u32;

        let (vertex_buffer, vertex_token) = device.create_buffer::<Vertex, _>(
			upload,
			gfx::buffer::Usage::VERTEX,
			vertex_count as u64
		).unwrap();

        let (index_buffer, index_token) = device.create_buffer::<u32, _>(
			upload,
			gfx::buffer::Usage::INDEX,
			index_count as u64
		).unwrap();

		device.write_mapping(&vertex_buffer, 0..vertex_count as u64)
			.unwrap()
			.copy_from_slice(&vertices[..]);

		device.write_mapping(&index_buffer, 0..index_count as u64)
			.unwrap()
			.copy_from_slice(&indices[..]);

		encoder.init_resources(vec![
			vertex_token,
			index_token,
		]);

        Model {
			vertex_buffer,
			vertex_count,
			index_buffer,
			index_count,
		}
    }
}

const QUAD: [Vertex; 6] = [
	Vertex { position: [ -0.5, 0.5 ] },
	Vertex { position: [  0.5, 0.5 ] },
	Vertex { position: [  0.5,-0.5 ] },

	Vertex { position: [ -0.5, 0.5 ] },
	Vertex { position: [  0.5,-0.5 ] },
	Vertex { position: [ -0.5,-0.5 ] },
];
