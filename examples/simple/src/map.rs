use std::sync::{ Arc, Mutex };
use opalite::{
    hal::{ self, Backend },
    cgmath::Vector3,
    renderer::model,
    specs::{ Entity, World },
    Backend as B,
    Buffer,
    InitialPosition,
    Model,
    ModelData,
    ModelKey,
    ModelType,
    ProceduralModel,
    RLock,
    Vertex,
    WLock,
};

pub struct HexMetrics { }

impl HexMetrics {
    pub const OUTER: f32 = 1.0;
    pub const INNER: f32 = Self::OUTER * 0.866025404;
    pub const CORNERS: [Vector3<f32>; 7] = [
        /*Vector3 { x: 0.0,           y: Self::OUTER,         z: 0.0 },
        Vector3 { x: Self::INNER,   y: 0.5 * Self::OUTER,   z: 0.0 },
        Vector3 { x: Self::INNER,   y: -0.5 * Self::OUTER,  z: 0.0 },
        Vector3 { x: 0.0,           y: -Self::OUTER,        z: 0.0 },
        Vector3 { x: -Self::INNER,  y: -0.5 * Self::OUTER,  z: 0.0 },
        Vector3 { x: -Self::INNER,  y: 0.5 * Self::OUTER,   z: 0.0 },
        // duplicate of first vector
        Vector3 { x: 0.0,           y: Self::OUTER,         z: 0.0 },*/


        Vector3 { x: 0.0,           y: 0.0, z: Self::OUTER },
        Vector3 { x: Self::INNER,   y: 0.0, z: 0.5 * Self::OUTER },
        Vector3 { x: Self::INNER,   y: 0.0, z: -0.5 * Self::OUTER },
        Vector3 { x: 0.0,           y: 0.0, z: -Self::OUTER },
        Vector3 { x: -Self::INNER,  y: 0.0, z: -0.5 * Self::OUTER },
        Vector3 { x: -Self::INNER,  y: 0.0, z: 0.5 * Self::OUTER },
        // duplicate of first vector
        Vector3 { x: 0.0,           y: 0.0, z: Self::OUTER },
    ];
}

pub struct HexGrid {
    pub width: i32,
    pub height: i32,
    cells: Vec<HexCell>,
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    model: Option<WLock<Model>>,
    needs_reload: bool,
}

impl HexGrid {
    pub fn new(width: i32, height: i32, world: &mut World) {
        let mut cells = vec![];
        let mut vertices = vec![];
        let mut indices = vec![];

        let mut i = 0;
        for z in 0..height {
            for x in 0..width {
                let cell = HexGrid::create_cell(x, z, i);
                cells.push(cell);

                i += 1;
            }
        }

        let mut grid = Self {
            width,
            height,
            cells,
            vertices,
            indices,
            model: None,
            needs_reload: false,
        };

        grid.triangulate();

        let _ = world.create_entity()
            .with(ModelKey::new(ModelType::Procedural(Arc::new(Mutex::new(grid)))))
            .with(ModelData {
                translate: Vector3::new(0.0, 0.0, 0.0),
                .. Default::default()
            })
            .build();
    }

    fn triangulate(&mut self) {
        self.vertices.clear();
        self.indices.clear();

        for cell in &self.cells.clone() {
            self.triangulate_single(&cell);
        }

        self.needs_reload = true;
    }

    fn triangulate_single(&mut self, cell: &HexCell) {
        for i in 0..6 {
            self.add_triangle(HexGrid::generate_triangle(
                cell.center,
                [
                    Vector3::new(0.0, 0.0, 0.0),
                    HexMetrics::CORNERS[i],
                    HexMetrics::CORNERS[i + 1],
                ]
            ));
        }
    }

    fn generate_triangle(base: Vertex, offsets: [Vector3<f32>; 3]) -> [Vertex; 3] {
        let map = |o| {
            let mut base = base;
            base.change_position(o);
            base
        };

        [
            map(offsets[0]),
            map(offsets[1]),
            map(offsets[2]),
        ]
    }

    fn add_triangle(&mut self, [v1, v2, v3]: [Vertex; 3]) {
        let current_index = self.vertices.len() as u32;

        self.vertices.push(v1);
        self.vertices.push(v2);
        self.vertices.push(v3);

        self.indices.push(current_index);
        self.indices.push(current_index + 1);
        self.indices.push(current_index + 2);
    }

    fn create_cell(x: i32, z: i32, i: i32) -> HexCell {
        let xf = x as f32;
        let zf = z as f32;
        let zf_2 = (z / 2) as f32;

        let xf = (xf + (zf * 0.5) - zf_2) * (HexMetrics::INNER * 2.0);
        let zf = zf * (HexMetrics::OUTER * 1.5);

        let position = Vector3::new(xf, 0.0, zf);
        let center = Vertex {
            position: position.into(),
            color: [0.5, 0.5, 1.0],
        };

        HexCell {
            center,
        }
    }
}

impl ProceduralModel for HexGrid {
    fn load(&mut self, device: Arc<Mutex<<B as Backend>::Device>>, memory_types: &[hal::MemoryType]) -> RLock<Model> {
        if self.model.is_some() {
            let model = self.model.as_ref().unwrap();
            let mut model = model.write().unwrap();
            model.vertex_buffer.write(&self.vertices[..]).unwrap();
            model.index_buffer.write(&self.indices[..]).unwrap();
        } else {
            let (mut vertex_buffer, mut index_buffer) = (
                Buffer::<Vertex, B>::new(device.clone(), self.vertices.len() as u64, hal::buffer::Usage::VERTEX, &memory_types).unwrap(),
                Buffer::<u32, B>::new(device.clone(), self.indices.len() as u64, hal::buffer::Usage::INDEX, &memory_types).unwrap(),
            );

            let model = WLock::new(Model {
                vertex_buffer,
                index_buffer,
            });

            self.model = Some(model);
        };

        self.model.as_ref().unwrap().get_reader()
    }

    fn needs_reload(&mut self) -> bool {
        if self.needs_reload == true {
            self.needs_reload = false;
            true
        } else {
            false
        }
    }
}

#[derive(Copy, Clone)]
pub struct HexCell {
    center: Vertex,
}

impl ProceduralModel for HexCell {
    fn load(&mut self, device: Arc<Mutex<<B as Backend>::Device>>, memory_types: &[hal::MemoryType]) -> RLock<Model> {
        let vertices = model::make_quad([0.5, 0.5, 1.0]).to_vec();

        let mut vertex_buffer = Buffer::<Vertex, B>::new(device.clone(), vertices.len() as u64, hal::buffer::Usage::VERTEX, &memory_types).unwrap();
        vertex_buffer.write(&vertices[..]).unwrap();

        let indices = (0..vertices.len() as u32).collect::<Vec<_>>();
        let mut index_buffer = Buffer::<u32, B>::new(device.clone(), indices.len() as u64, hal::buffer::Usage::INDEX, &memory_types).unwrap();
        index_buffer.write(&indices[..]).unwrap();

        RLock::new(Model {
            vertex_buffer,
            index_buffer,
        })
    }
}
