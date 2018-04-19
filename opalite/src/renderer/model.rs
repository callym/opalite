use std::{
    self,
    cmp::{ Eq, PartialEq },
    collections::HashMap,
    fs,
    hash::{ Hash, Hasher },
    io,
    path::PathBuf,
    sync::{ Arc, Mutex },
};
use back::Backend as B;
use failure::Error;
use genmesh::{ self, generators::{ self, IndexedPolygon, SharedVertex }, Triangulate, Vertices };
use hal::{ self, Backend };
use cgmath::{ prelude::*, Matrix4, Vector2, Vector3, Vector4 };
use ordered_float::NotNaN;
use gltf::{ self, json::mesh::Mode };
use gltf_importer;
use gltf_utils::PrimitiveIterators;
use uuid::Uuid;
use crate::{ renderer::{ Buffer, BufferData }, Resources, RLock };
use crate::renderer::conv::*;

#[derive(BufferData, Copy, Clone, Debug)]
pub struct Vertex {
    pub position: Vector3<f32>,
    pub color: Vector4<f32>,
    pub uv: Vector2<f32>,
    pub normal: Vector3<f32>,
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
            normal: Vector3::zero(),
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
    fn load(&mut self, device: Arc<Mutex<<B as Backend>::Device>>, memory_types: &[hal::MemoryType]) -> Vec<RLock<Model>>;
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
    pub fn calculate_normals(mut vertices: Vec<Vertex>, indices: Vec<u32>) -> (Vec<Vertex>, Vec<u32>) {
        for chunk in indices.chunks(3) {
            let i1 = chunk[0];
            let i2 = chunk[2];
            let i3 = chunk[1];

            let mut v1 = vertices[i1 as usize];
            let mut v2 = vertices[i2 as usize];
            let mut v3 = vertices[i3 as usize];

            let diff_1_2 = v1.position - v2.position;
            let diff_1_2 = diff_1_2.normalize();
            let diff_3_2 = v3.position - v2.position;
            let diff_3_2 = diff_3_2.normalize();

            let normal = diff_1_2.cross(diff_3_2);
            let normal = normal.normalize();
            v1.normal = normal;
            v2.normal = normal;
            v3.normal = normal;

            vertices[i1 as usize] = v1;
            vertices[i2 as usize] = v2;
            vertices[i3 as usize] = v3;
        }

        (vertices, indices)
    }

    pub fn from_file(path: &PathBuf, resources: &RLock<Resources>, device: Arc<Mutex<<B as Backend>::Device>>, memory_types: &[hal::MemoryType]) -> Result<Vec<RLock<Self>>, Error> {
        let resources = resources.read().unwrap();
        let gltf = resources.get(path)?;
        let (gltf, buffers) = gltf_importer::import_data_slice(&gltf[..], path, &Default::default())?;

        let mut output_meshes = vec![];

        for mesh in gltf.meshes() {
            let primitive = mesh.primitives().next().ok_or(format_err!(".gltf doesn't contain any primitives"))?;

            let positions = primitive.positions(&buffers).ok_or(format_err!("primitive doesn't have positions"))?;
            let mut vertices: Vec<_> = positions
                .map(|p| Vertex { position: p.into(), .. Default::default() })
                .collect();

            let indices: Vec<_> = if let Some(indices) = <_ as PrimitiveIterators>::indices(&primitive, &buffers) {
                indices.into_u32().collect()
            } else {
                (0 .. vertices.len() as u32).collect()
            };

            let (mut vertices, indices) = if let Some(normals) = primitive.normals(&buffers) {
                for (i, normal) in normals.enumerate() {
                    vertices[i].normal = normal.into();
                }
                (vertices, indices)
            } else {
                Model::calculate_normals(vertices, indices)
            };

            if let Some(colors) = primitive.colors(0, &buffers) {
                let colors = colors.into_rgba_f32();
                for (i, color) in colors.enumerate() {
                    vertices[i].color = color.into();
                }
            }

            if let Some(uvs) = primitive.tex_coords(0, &buffers) {
                let uvs = uvs.into_f32();
                for (i, uv) in uvs.enumerate() {
                    vertices[i].uv = uv.into();
                }
            }

            let mut vertex_buffer = Buffer::<Vertex, B>::new(device.clone(), vertices.len() as u64, hal::buffer::Usage::VERTEX, &memory_types).unwrap();
            vertex_buffer.write(&vertices[..]).unwrap();

            let mut index_buffer = Buffer::<u32, B>::new(device.clone(), indices.len() as u64, hal::buffer::Usage::INDEX, &memory_types).unwrap();
            index_buffer.write(&indices[..]).unwrap();

            output_meshes.push(RLock::new(Self {
                vertex_buffer,
                index_buffer,
            }));
        }

        Ok(output_meshes)
    }

    pub(crate) fn quad<C: Into<Vector4<f32>>>(color: C, device: Arc<Mutex<<B as Backend>::Device>>, memory_types: &[hal::MemoryType], calculate_normals: bool) -> RLock<Self> {
        let vertices = make_quad(color).to_vec();
        let indices = (0..6 as u32).collect::<Vec<_>>();

        let (vertices, indices) = if calculate_normals {
            Model::calculate_normals(vertices, indices)
        } else {
            (vertices, indices)
        };

        let mut vertex_buffer = Buffer::<Vertex, B>::new(device.clone(), vertices.len() as u64, hal::buffer::Usage::VERTEX, &memory_types).unwrap();
        vertex_buffer.write(&vertices[..]).unwrap();

        let mut index_buffer = Buffer::<u32, B>::new(device.clone(), indices.len() as u64, hal::buffer::Usage::INDEX, &memory_types).unwrap();
        index_buffer.write(&indices[..]).unwrap();

        RLock::new(Self {
            vertex_buffer,
            index_buffer,
        })
    }

    pub(crate) fn hex<C: Into<Vector4<f32>>>(color: C, device: Arc<Mutex<<B as Backend>::Device>>, memory_types: &[hal::MemoryType], calculate_normals: bool) -> RLock<Self> {
        let (vertices, indices) = make_hex(color);
        let vertices = vertices.to_vec();
        let indices = indices.to_vec();

        let (vertices, indices) = if calculate_normals {
            Model::calculate_normals(vertices, indices)
        } else {
            (vertices, indices)
        };

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

    Vertex { position: Vector3::new(-0.5, 0.0, 0.5), color, uv: Vector2::new(0.0, 1.0), .. Default::default() },
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
    let color: Vector4<_> = color.into();
    let generator = generators::IcoSphere::subdivide(1);

    let vertices = generator
        .shared_vertex_iter()
        .map(|v| Vertex {
            position: v.pos.into(),
            color,
            normal: v.normal.into(),
            ..
            Default::default()
        })
        .collect();

    let indices = generator
        .indexed_polygon_iter()
        .triangulate()
        .vertices()
        .map(|i| i as u32)
        .collect();

    (vertices, indices)
}
