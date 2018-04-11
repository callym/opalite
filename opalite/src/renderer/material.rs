use std::{ collections::HashMap, mem, sync::{ Arc, Mutex } };
use crate::renderer::{ Buffer, BufferData, ImageKey, Image, PushConstant };
use crate::renderer::conv::*;

use ordered_float::NotNaN;
use hal::{ pso, Backend, DescriptorPool, Device };
use hal::pso::{ PipelineStage, ShaderStageFlags };
use back::Backend as B;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum SurfaceType {
    Color([NotNaN<f32>; 4]),
    Texture(ImageKey),
}

#[derive(Debug, Component, PartialEq, Eq, Clone, Hash)]
pub struct MaterialDesc {
    pub diffuse: SurfaceType,
}

impl MaterialDesc {
    pub fn fallback() -> Self {
        Self {
            diffuse: SurfaceType::Color(vec4(1.0, 0.0, 1.0, 0.0)),
        }
    }
}

pub struct Material {
    pub diffuse: SurfaceType,
    pub descriptor_set: <B as Backend>::DescriptorSet,
}

impl Material {
    pub fn new<'a>(material: MaterialDesc, images: &HashMap<ImageKey, Image<B>>, device: Arc<Mutex<<B as Backend>::Device>>) -> Self {
        Self {
            diffuse: material.diffuse.clone(),
            descriptor_set: Material::descriptor_set(material, images, device),
        }
    }

    pub fn set_layout(device: Arc<Mutex<<B as Backend>::Device>>) -> <B as Backend>::DescriptorSetLayout {
        let device = device.lock().unwrap();
        device.create_descriptor_set_layout(&Image::<B>::descriptor_set_binding(
            ShaderStageFlags::FRAGMENT,
            0,
        )[..])
    }

    fn descriptor_set<'a>(material: MaterialDesc, images: &HashMap<ImageKey, Image<B>>, device: Arc<Mutex<<B as Backend>::Device>>) -> <B as Backend>::DescriptorSet {
        let set_layout = Material::set_layout(device.clone());

        let device = device.lock().unwrap();

        let mut desc_pool = {
            let mut desc_range = vec![];

            // diffuse
            desc_range.extend(Image::<B>::descriptor_range());

            device.create_descriptor_pool(1, &desc_range[..])
        };

        let desc_set = desc_pool.allocate_set(&set_layout);

        let mut desc_set_write = vec![];

        if let SurfaceType::Texture(key) = material.diffuse {
            if let Some(image) = images.get(&key) {
                desc_set_write.extend(image.descriptor_set(0, &desc_set));
            } else {
                let image = images.get(&ImageKey(String::from("Blank"))).unwrap();
                desc_set_write.extend(image.descriptor_set(0, &desc_set));
            }
        } else {
            let image = images.get(&ImageKey(String::from("Blank"))).unwrap();
            desc_set_write.extend(image.descriptor_set(0, &desc_set));
        }

        device.write_descriptor_sets(desc_set_write);

        desc_set
    }
}

#[derive(Serialize)]
struct MaterialData {
    diffuse: [f32; 4],
}

impl PushConstant for Material {
    const SIZE: u32 = (mem::size_of::<MaterialData>() / 4) as u32;

    fn data(&self) -> Vec<u32> {
        use ::bincode::serialize;

        let diffuse = match self.diffuse {
            SurfaceType::Color(color) => [*color[0], *color[1], *color[2], *color[3]],
            SurfaceType::Texture(_) => [1.0; 4],
        };

        let data = serialize(&MaterialData {
            diffuse,
        }).unwrap();

        data.chunks(4).map(|d| {
            ((d[0] as u32) << 0) |
            ((d[1] as u32) << 8) |
            ((d[2] as u32) << 16) |
            ((d[3] as u32) << 24)
        }).collect()
    }
}
