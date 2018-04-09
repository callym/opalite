use crate::renderer::{ Buffer, BufferData, ModelData };
use crate::renderer::conv::*;

use cgmath::Vector3;

use ordered_float::NotNaN;
use hal::{ pso, Backend, DescriptorPool, Device };
use back::Backend as B;

#[derive(Debug, Clone)]
pub enum LightType {
    None,
    Point,
}

#[derive(Debug, Component, Clone)]
pub struct Light {
    pub ty: LightType,
    pub color: Vector3<f32>,
}

impl Light {
    pub(super) fn to_data(&self, model_data: ModelData) -> LightData {
        let ty = match self.ty {
            LightType::None => 0,
            LightType::Point => 1,
        };

        LightData {
            ty,
            color: self.color,
            position: model_data.translate,
        }
    }
}

#[derive(BufferData, Clone, Copy, Debug, Serialize)]
#[uniform]
pub struct LightData {
    pub ty: u32,
    pub color: Vector3<f32>,
    pub position: Vector3<f32>,
}

impl Default for LightData {
    fn default() -> Self {
        Self {
            ty: 0,
            color: Vector3::new(1.0, 1.0, 1.0),
            position: Vector3::new(0.0, 0.0, 0.0),
        }
    }
}
