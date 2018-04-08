use std::{
    self,
    cmp::{ Eq, PartialEq },
    hash::{ Hash, Hasher },
    path::PathBuf,
    sync::{ Arc, Mutex },
};
use back::Backend as B;
use hal::{ self, Backend };
use cgmath::{ prelude::*, Matrix4, Vector2, Vector3, Vector4 };
use uuid::Uuid;
use crate::{ renderer::{ Buffer, BufferData }, RLock };

#[derive(BufferData, Copy, Clone, Debug)]
pub struct Vertex {
    pub position: Vector3<f32>,
    pub color: Vector4<f32>,
    pub uv: Vector2<f32>,
}

#[derive(BufferData, Copy, Clone, Debug)]
pub struct UiVertex {
    pub position: Vector2<f32>,
    pub color: Vector4<f32>,
    pub uv: Vector2<f32>,
    pub mode: u32,
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            position: Vector3::zero(),
            color: Vector4::new(1.0, 1.0, 1.0, 1.0),
            uv: Vector2::zero(),
        }
    }
}

#[derive(Component, Clone)]
pub struct ModelKey(ModelType, Uuid);

impl ModelKey {
    pub fn new(model_type: ModelType) -> Self {
        ModelKey(model_type, Uuid::new_v4())
    }

    pub fn id(&self) -> Uuid {
        self.1
    }

    pub fn ty(&self) -> &ModelType {
        &self.0
    }

    pub fn ty_mut(&mut self) -> &mut ModelType {
        &mut self.0
    }
}

impl Hash for ModelKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

impl PartialEq for ModelKey {
    fn eq(&self, other: &ModelKey) -> bool {
        self.id() == other.id()
    }
}

impl Eq for ModelKey { }

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
            scale: Vector3::new(1.0, 1.0, 1.0),
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

pub trait ProceduralModel {
    fn load(&mut self, device: Arc<Mutex<<B as Backend>::Device>>, memory_types: &[hal::MemoryType]) -> RLock<Model>;
    fn needs_reload(&mut self) -> bool {
        false
    }
}

#[derive(Clone)]
pub enum ModelType {
    Quad,
    Hex,
    Sphere,
    Procedural(Arc<Mutex<ProceduralModel + Send + Sync>>),
    File(PathBuf),
}

pub struct Model<V: BufferData = Vertex> {
    pub vertex_buffer: Buffer<V, B>,
    pub index_buffer: Buffer<u32, B>,
}

impl Model {
    pub(crate) fn quad<C: Into<Vector4<f32>>>(color: C, device: Arc<Mutex<<B as Backend>::Device>>, memory_types: &[hal::MemoryType]) -> RLock<Self> {
        let vertices = make_quad(color).to_vec();
        let mut vertex_buffer = Buffer::<Vertex, B>::new(device.clone(), vertices.len() as u64, hal::buffer::Usage::VERTEX, &memory_types).unwrap();
        vertex_buffer.write(&vertices[..]).unwrap();

        let indices = (0..6 as u32).collect::<Vec<_>>();
        let mut index_buffer = Buffer::<u32, B>::new(device.clone(), indices.len() as u64, hal::buffer::Usage::INDEX, &memory_types).unwrap();
        index_buffer.write(&indices[..]).unwrap();

        RLock::new(Self {
            vertex_buffer,
            index_buffer,
        })
    }

    pub(crate) fn hex<C: Into<Vector4<f32>>>(color: C, device: Arc<Mutex<<B as Backend>::Device>>, memory_types: &[hal::MemoryType]) -> RLock<Self> {
        let (vertices, indices) = make_hex(color);

        let mut vertex_buffer = Buffer::<Vertex, B>::new(device.clone(), vertices.len() as u64, hal::buffer::Usage::VERTEX, &memory_types).unwrap();
        vertex_buffer.write(&vertices[..]).unwrap();

        let mut index_buffer = Buffer::<u32, B>::new(device.clone(), indices.len() as u64, hal::buffer::Usage::INDEX, &memory_types).unwrap();
        index_buffer.write(&indices[..]).unwrap();

        RLock::new(Self {
            vertex_buffer,
            index_buffer,
        })
    }

    pub(crate) fn sphere<C: Into<Vector4<f32>>>(color: C, device: Arc<Mutex<<B as Backend>::Device>>, memory_types: &[hal::MemoryType]) -> RLock<Self> {
        let (vertices, indices) = make_sphere(color);

        let mut vertex_buffer = Buffer::<Vertex, B>::new(device.clone(), vertices.len() as u64, hal::buffer::Usage::VERTEX, &memory_types).unwrap();
        vertex_buffer.write(&vertices[..]).unwrap();

        let mut index_buffer = Buffer::<u32, B>::new(device.clone(), indices.len() as u64, hal::buffer::Usage::INDEX, &memory_types).unwrap();
        index_buffer.write(&indices[..]).unwrap();

        RLock::new(Self {
            vertex_buffer,
            index_buffer,
        })
    }
}

pub fn make_quad<C: Into<Vector4<f32>>>(color: C) -> [Vertex; 6] {
    let color = color.into();
    [
    Vertex { position: Vector3::new(-0.5, 0.0, 0.5), color, uv: Vector2::new(0.0, 1.0), .. Default::default() },
    Vertex { position: Vector3::new( 0.5, 0.0, 0.5), color, uv: Vector2::new(1.0, 1.0), .. Default::default() },
    Vertex { position: Vector3::new( 0.5, 0.0,-0.5), color, uv: Vector2::new(1.0, 0.0), .. Default::default() },

    Vertex { position: Vector3::new(-0.5, 0.5, 0.5), color, uv: Vector2::new(0.0, 1.0), .. Default::default() },
    Vertex { position: Vector3::new( 0.5, 0.0,-0.5), color, uv: Vector2::new(1.0, 0.0), .. Default::default() },
    Vertex { position: Vector3::new(-0.5, 0.0,-0.5), color, uv: Vector2::new(0.0, 0.0), .. Default::default() },
    ]
}

pub fn make_hex<C: Into<Vector4<f32>>>(color: C) -> ([Vertex; 7], [u32; 18]) {
    let start_angle = 0.5;

    let offset = |corner| {
        let angle: f32 = 2.0 * std::f32::consts::PI * (start_angle + corner) / 6.0;
        (angle.cos(), angle.sin())
    };

    let color: Vector4<_> = color.into();

    let mut arr: [Vertex; 7] = Default::default();
    for (i, v) in arr.iter_mut().enumerate() {
        v.color = color;

        if i == 0 {
            v.position.x = 0.0;
            v.position.z = 0.0;
            continue;
        }

        let (x, z) = offset(i as f32);
        v.position.x = x;
        v.position.z = z;
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

pub fn make_sphere<C: Into<Vector4<f32>>>(color: C) -> (Vec<Vertex>, Vec<u32>) {
    let x = 0.525731112119133606;
    let z = 0.850650808352039932;
    let n = 0.0;

    let icosahedron = vec![
        [-x, n, z], [x, n, z], [-x, n,-z], [x, n,-z],
        [n, z, x], [n, z,-x], [n,-z, x], [n,-z,-x],
        [z, x, n], [-z, x, n], [z,-x, n], [-z,-x, n],
    ];

    let indices = vec![
        0, 4, 1, 0, 9, 4, 9, 5, 4, 4, 5, 8, 4, 8, 1,
        8, 10, 1, 8, 3, 10, 5, 3, 8, 5, 2, 3, 2, 7, 3,
        7, 10, 3, 7, 6, 10, 7, 11, 6, 11, 0, 6, 0, 1, 6,
        6, 1, 10, 9, 0, 11, 9, 11, 2, 9, 2, 5, 7, 2, 11,
    ];

    let color: Vector4<_> = color.into();
    let vertices = icosahedron.iter().map(|v| Vertex {
        position: (*v).into(),
        color,
        .. Default::default()
    }).collect();

    (vertices, indices)
}
