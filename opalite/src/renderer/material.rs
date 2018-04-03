use std::{ collections::HashMap, mem, sync::{ Arc, Mutex } };
use crate::renderer::{ Buffer, BufferData, ImageKey, Image, PushConstant };
use crate::renderer::conv::*;

use ordered_float::NotNaN;
use hal::{ self, pso, Backend, DescriptorPool, Device };

use back;
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
    pub fn new<L: BufferData>(material: MaterialDesc, images: &HashMap<ImageKey, Image<B>>, locals: &Buffer<L, B>, device: Arc<Mutex<<B as Backend>::Device>>, set_layout: &<B as Backend>::DescriptorSetLayout) -> Self {
        Self {
            diffuse: material.diffuse.clone(),
            descriptor_set: Material::descriptor_set(material, images, locals, device, set_layout),
        }
    }

    fn descriptor_set<L: BufferData>(material: MaterialDesc, images: &HashMap<ImageKey, Image<B>>, locals: &Buffer<L, B>, device: Arc<Mutex<<B as Backend>::Device>>, set_layout: &<B as Backend>::DescriptorSetLayout) -> <B as Backend>::DescriptorSet {
        let device = device.lock().unwrap();

        let mut desc_pool = {
            let mut desc = vec![
                pso::DescriptorRangeDesc {
                    ty: pso::DescriptorType::UniformBuffer,
                    count: 1,
                }
            ];
            // diffuse
            desc.extend(Image::<B>::descriptor_range());

            device.create_descriptor_pool(1, &desc[..])
        };

        let desc_set = desc_pool.allocate_set(&set_layout);

        let mut desc_set_write = vec![locals.descriptor_set(0, 0, &desc_set)];

        if let SurfaceType::Texture(key) = material.diffuse {
            if let Some(image) = images.get(&key) {
                desc_set_write.extend(image.descriptor_set(1, &desc_set));
            } else {
                let image = images.get(&ImageKey(String::from("Blank"))).unwrap();
                desc_set_write.extend(image.descriptor_set(1, &desc_set));
            }
        } else {
            let image = images.get(&ImageKey(String::from("Blank"))).unwrap();
            desc_set_write.extend(image.descriptor_set(1, &desc_set));
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

        unsafe { mem::transmute::<Vec<u8>, Vec<u32>>(data) }
    }
}
