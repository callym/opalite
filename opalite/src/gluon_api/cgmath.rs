use cgmath;
use gluon::{
    self,
    vm::{
        self,
        api::{ Getable, Userdata, ValueRef, VmType },
        gc::{ Gc, Traverseable },
        Variants,
    },
    Thread,
};

#[derive(Debug, Clone, Copy)]
pub struct Vec3(pub cgmath::Vector3<f64>);

register_gluon!(Vec3);

impl Vec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Vec3(cgmath::Vector3::new(x, y, z))
    }

    pub fn x(&self) -> f64 {
        self.0.x
    }

    pub fn y(&self) -> f64 {
        self.0.y
    }

    pub fn z(&self) -> f64 {
        self.0.z
    }
}

pub fn register_opalite_api(vm: &gluon::Thread) {
    vm.register_type::<Vec3>("Vec3", &[]).unwrap();

    gluon::import::add_extern_module(vm, "cgmath", |vm: &gluon::Thread| {
        vm::ExternModule::new(vm, record!(
            vec3 => record!(
                new => primitive!(3 Vec3::new),
                x => primitive!(1 Vec3::x),
                y => primitive!(1 Vec3::y),
                z => primitive!(1 Vec3::z),
            ),
        ))
    });
}
