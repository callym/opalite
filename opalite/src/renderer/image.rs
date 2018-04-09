use std::{
    marker::PhantomData,
    path::PathBuf,
    sync::{ Arc, Mutex },
};
use failure::Error;
use hal::{
    self,
    buffer,
    command,
    format::{
        self as f,
        AsFormat,
        Rgba8Srgb as ColorFormat,
        Swizzle,
    },
    image as i,
    memory as m,
    pso::{
        self,
        PipelineStage,
        ShaderStageFlags,
    },
    Backend, Device,
};
use image;
use crate::renderer;

pub struct Sampler<B: Backend>(<B as Backend>::Sampler);

impl<B: Backend> Sampler<B> {
    pub fn new(sampler: <B as Backend>::Sampler) -> Self {
        Sampler(sampler)
    }

    pub fn sampler(&self) -> &<B as Backend>::Sampler {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageKey(pub String);

pub struct Image<B: Backend> {
    row_pitch: u32,
    row_alignment_mask: u32,
    stride: u32,
    dimensions: (u32, u32),
    image: <B as Backend>::Image,
    image_upload_buffer: <B as Backend>::Buffer,
    image_upload_memory: <B as Backend>::Memory,
    pub sampler: Arc<Sampler<B>>,
    pub srv: <B as Backend>::ImageView,
    pub submitted: bool,
    _phantom: PhantomData<B>,
}

impl<B: Backend> Image<B> {
    pub fn descriptor_set_binding(flags: ShaderStageFlags, offset: u32) -> Vec<pso::DescriptorSetLayoutBinding> {
        vec![
            pso::DescriptorSetLayoutBinding {
                binding: offset,
                ty: pso::DescriptorType::SampledImage,
                count: 1,
                stage_flags: flags,
            },
            pso::DescriptorSetLayoutBinding {
                binding: offset + 1,
                ty: pso::DescriptorType::Sampler,
                count: 1,
                stage_flags: flags,
            },
        ]
    }

    pub fn descriptor_range() -> Vec<pso::DescriptorRangeDesc> {
        vec![
            pso::DescriptorRangeDesc {
                ty: pso::DescriptorType::SampledImage,
                count: 1,
            },
            pso::DescriptorRangeDesc {
                ty: pso::DescriptorType::Sampler,
                count: 1,
            },
        ]
    }

    pub fn descriptor_set<'a>(&'a self, offset: u32, desc_set: &'a <B as Backend>::DescriptorSet) -> Vec<pso::DescriptorSetWrite<'a, B, Option<pso::Descriptor<B>>>> {
        vec![
            pso::DescriptorSetWrite {
                set: desc_set,
                binding: offset,
                array_offset: 0,
                descriptors: Some(
                    pso::Descriptor::Image(&self.srv, i::Layout::Undefined)
                ),
            },
            pso::DescriptorSetWrite {
                set: desc_set,
                binding: offset + 1,
                array_offset: 0,
                descriptors: Some(
                    pso::Descriptor::Sampler(&*self.sampler.sampler())
                ),
            },
        ]
    }

    pub fn update(&mut self, offset: [usize; 2], size: [usize; 2], image_data: &[[u8; 4]], device: Arc<Mutex<B::Device>>) -> Result<(), Error> {
        let device = device.lock().unwrap();
        let (width, height) = self.dimensions;
        let row_pitch = self.row_pitch;
        let upload_size = (height * row_pitch) as u64;

        // copy image data into staging buffer
        {
            let mut data = device.acquire_mapping_writer::<u8>(&self.image_upload_memory, 0..upload_size)?;
            for y in 0..size[1] as usize {
                let x = size[0];
                let dest_base = (y + offset[1]) * row_pitch as usize;
                let dest_base = dest_base + (offset[0] * 4);

                let row = &(*image_data)[y * x .. (y + 1) * x];
                let row = row.iter().flat_map(|r| r).map(|r| *r).collect::<Vec<_>>();

                data[dest_base .. dest_base + row.len()].copy_from_slice(&row[..]);
            }

            device.release_mapping_writer(data);
        }

        self.submitted = false;

        Ok(())
    }

    pub fn from_data(key: String, width: u32, height: u32, image_data: &[[u8; 4]], limits: &hal::Limits, device: Arc<Mutex<B::Device>>, memory_types: &[hal::MemoryType], sampler: Arc<Sampler<B>>) -> Result<(ImageKey, Self), Error> {
        let device = device.lock().unwrap();

        let kind = i::Kind::D2(width as i::Size, height as i::Size, 1, 1);
        let row_alignment_mask = limits.min_buffer_copy_pitch_alignment as u32 - 1;
        let image_stride = 4_usize;
        let row_pitch = (width * image_stride as u32 + row_alignment_mask) & !row_alignment_mask;
        let upload_size = (height * row_pitch) as u64;

        let image_buffer_unbound = device.create_buffer(upload_size, buffer::Usage::TRANSFER_SRC)?;
        let image_mem_reqs = device.get_buffer_requirements(&image_buffer_unbound);
        let upload_type = memory_types
            .iter()
            .enumerate()
            .position(|(id, mem_type)| {
                image_mem_reqs.type_mask & (1 << id) != 0 &&
                mem_type.properties.contains(m::Properties::CPU_VISIBLE)
            })
            .unwrap()
            .into();
        let image_upload_memory = device.allocate_memory(upload_type, image_mem_reqs.size)?;
        let image_upload_buffer = device.bind_buffer_memory(&image_upload_memory, 0, image_buffer_unbound)?;

        // copy image data into staging buffer
        {
            let mut data = device.acquire_mapping_writer::<u8>(&image_upload_memory, 0..upload_size)?;
            for y in 0..height as usize {
                let row = &(*image_data)[y * (width as usize) .. (y + 1) * (width as usize)];
                let row = row.iter().flat_map(|r| r).map(|r| *r).collect::<Vec<_>>();
                let dest_base = y * row_pitch as usize;
                data[dest_base .. dest_base + row.len()].copy_from_slice(&row[..]);
            }
            device.release_mapping_writer(data);
        }

        // TODO: usage
        let image_unbound = device.create_image(
            kind,
            1,
            ColorFormat::SELF,
            i::Tiling::Optimal,
            i::Usage::TRANSFER_DST | i::Usage::SAMPLED,
            i::StorageFlags::empty(),
        )?;
        let image_req = device.get_image_requirements(&image_unbound);
        let device_type = memory_types
            .iter()
            .enumerate()
            .position(|(id, memory_type)| image_req.type_mask & (1 << id) != 0 && memory_type.properties.contains(m::Properties::DEVICE_LOCAL))
            .unwrap().into();
        let image_memory = device.allocate_memory(device_type, image_req.size)?;
        let image = device.bind_image_memory(&image_memory, 0, image_unbound)?;
        let srv = device.create_image_view(&image, i::ViewKind::D2, ColorFormat::SELF, Swizzle::NO, renderer::COLOR_RANGE.clone())?;

        let image = Self {
            row_pitch,
            row_alignment_mask,
            stride: image_stride as u32,
            dimensions: (width, height),
            image,
            image_upload_buffer,
            image_upload_memory,
            sampler,
            srv,
            submitted: false,
            _phantom: PhantomData,
        };

        let key = ImageKey(key);

        Ok((key, image))
    }

    pub fn blank(limits: &hal::Limits, device: Arc<Mutex<B::Device>>, memory_types: &[hal::MemoryType], sampler: Arc<Sampler<B>>) -> Result<(ImageKey, Self), Error> {
        let device = device.lock().unwrap();

        let (width, height) = (1, 1);
        let kind = i::Kind::D2(width as i::Size, height as i::Size, 1, 1);
        let row_alignment_mask = limits.min_buffer_copy_pitch_alignment as u32 - 1;
        let image_stride = 4_usize;
        let row_pitch = (width * image_stride as u32 + row_alignment_mask) & !row_alignment_mask;
        let upload_size = (height * row_pitch) as u64;

        let image_buffer_unbound = device.create_buffer(upload_size, buffer::Usage::TRANSFER_SRC)?;
        let image_mem_reqs = device.get_buffer_requirements(&image_buffer_unbound);
        let upload_type = memory_types
            .iter()
            .enumerate()
            .position(|(id, mem_type)| {
                image_mem_reqs.type_mask & (1 << id) != 0 &&
                mem_type.properties.contains(m::Properties::CPU_VISIBLE)
            })
            .unwrap()
            .into();
        let image_upload_memory = device.allocate_memory(upload_type, image_mem_reqs.size)?;
        let image_upload_buffer = device.bind_buffer_memory(&image_upload_memory, 0, image_buffer_unbound)?;

        // copy image data into staging buffer
        {
            let mut data = device.acquire_mapping_writer::<u8>(&image_upload_memory, 0..upload_size)?;
            data[0 .. 4].copy_from_slice(&[255, 255, 255, 255]);
            device.release_mapping_writer(data);
        }

        // TODO: usage
        let image_unbound = device.create_image(
            kind,
            1,
            ColorFormat::SELF,
            i::Tiling::Optimal,
            i::Usage::TRANSFER_DST | i::Usage::SAMPLED,
            i::StorageFlags::empty(),
        )?;
        let image_req = device.get_image_requirements(&image_unbound);
        let device_type = memory_types
            .iter()
            .enumerate()
            .position(|(id, memory_type)| image_req.type_mask & (1 << id) != 0 && memory_type.properties.contains(m::Properties::DEVICE_LOCAL))
            .unwrap().into();
        let image_memory = device.allocate_memory(device_type, image_req.size)?;
        let image = device.bind_image_memory(&image_memory, 0, image_unbound)?;
        let srv = device.create_image_view(&image, i::ViewKind::D2, ColorFormat::SELF, Swizzle::NO, renderer::COLOR_RANGE.clone())?;

        let image = Self {
            row_pitch,
            row_alignment_mask,
            stride: image_stride as u32,
            dimensions: (width, height),
            image,
            image_upload_buffer,
            image_upload_memory,
            sampler,
            srv,
            submitted: false,
            _phantom: PhantomData,
        };

        let key = ImageKey(String::from("Blank"));

        Ok((key, image))
    }

    pub fn new<P>(filename: P, limits: &hal::Limits, device: Arc<Mutex<B::Device>>, memory_types: &[hal::MemoryType], sampler: Arc<Sampler<B>>) -> Result<(ImageKey, Self), Error>
        where P: Into<PathBuf> + Into<String> + Clone
    {
        let device = device.lock().unwrap();

        let path: PathBuf = filename.clone().into();
        let img = image::open(path)?.to_rgba();
        let (width, height) = img.dimensions();
        let kind = i::Kind::D2(width as i::Size, height as i::Size, 1, 1);
        let row_alignment_mask = limits.min_buffer_copy_pitch_alignment as u32 - 1;
        let image_stride = 4_usize;
        let row_pitch = (width * image_stride as u32 + row_alignment_mask) & !row_alignment_mask;
        let upload_size = (height * row_pitch) as u64;

        let image_buffer_unbound = device.create_buffer(upload_size, buffer::Usage::TRANSFER_SRC)?;
        let image_mem_reqs = device.get_buffer_requirements(&image_buffer_unbound);
        let upload_type = memory_types
            .iter()
            .enumerate()
            .position(|(id, mem_type)| {
                image_mem_reqs.type_mask & (1 << id) != 0 &&
                mem_type.properties.contains(m::Properties::CPU_VISIBLE)
            })
            .unwrap()
            .into();
        let image_upload_memory = device.allocate_memory(upload_type, image_mem_reqs.size)?;
        let image_upload_buffer = device.bind_buffer_memory(&image_upload_memory, 0, image_buffer_unbound)?;

        // copy image data into staging buffer
        {
            let mut data = device.acquire_mapping_writer::<u8>(&image_upload_memory, 0..upload_size)?;
            for y in 0..height as usize {
                let row = &(*img)[y * (width as usize) * image_stride .. (y + 1) * (width as usize) * image_stride];
                let dest_base = y * row_pitch as usize;
                data[dest_base .. dest_base + row.len()].copy_from_slice(row);
            }
            device.release_mapping_writer(data);
        }

        // TODO: usage
        let image_unbound = device.create_image(
            kind,
            1,
            ColorFormat::SELF,
            i::Tiling::Optimal,
            i::Usage::TRANSFER_DST | i::Usage::SAMPLED,
            i::StorageFlags::empty(),
        )?;
        let image_req = device.get_image_requirements(&image_unbound);
        let device_type = memory_types
            .iter()
            .enumerate()
            .position(|(id, memory_type)| image_req.type_mask & (1 << id) != 0 && memory_type.properties.contains(m::Properties::DEVICE_LOCAL))
            .unwrap().into();
        let image_memory = device.allocate_memory(device_type, image_req.size)?;
        let image = device.bind_image_memory(&image_memory, 0, image_unbound)?;
        let srv = device.create_image_view(&image, i::ViewKind::D2, ColorFormat::SELF, Swizzle::NO, renderer::COLOR_RANGE.clone())?;

        let image = Self {
            row_pitch,
            row_alignment_mask,
            stride: image_stride as u32,
            dimensions: (width, height),
            image,
            image_upload_buffer,
            image_upload_memory,
            sampler,
            srv,
            submitted: false,
            _phantom: PhantomData,
        };

        let key = ImageKey(filename.into());

        Ok((key, image))
    }

    pub fn submit(&mut self, command_buffer: &mut command::CommandBuffer<B, hal::Graphics>) {
        self.submitted = true;

        let Self { image, image_upload_buffer, row_pitch, stride, dimensions, .. } = self;
        let (width, height) = dimensions;

        let image_barrier = m::Barrier::Image {
            states: (i::Access::empty(), i::Layout::Undefined) ..
                    (i::Access::TRANSFER_WRITE, i::Layout::TransferDstOptimal),
            target: image,
            range: renderer::COLOR_RANGE.clone(),
        };

        command_buffer.pipeline_barrier(
            PipelineStage::TOP_OF_PIPE .. PipelineStage::TRANSFER,
            m::Dependencies::empty(),
            &[image_barrier],
        );

        command_buffer.copy_buffer_to_image(
            &image_upload_buffer,
            &image,
            i::Layout::TransferDstOptimal,
            &[command::BufferImageCopy {
                buffer_offset: 0,
                buffer_width: *row_pitch / *stride,
                buffer_height: *height,
                image_layers: i::SubresourceLayers {
                    aspects: f::Aspects::COLOR,
                    level: 0,
                    layers: 0 .. 1,
                },
                image_offset: i::Offset { x: 0, y: 0, z: 0 },
                image_extent: i::Extent { width: *width, height: *height, depth: 1 },
            }]);

        let image_barrier = m::Barrier::Image {
            states: (i::Access::TRANSFER_WRITE, i::Layout::TransferDstOptimal) ..
                    (i::Access::SHADER_READ, i::Layout::ShaderReadOnlyOptimal),
            target: image,
            range: renderer::COLOR_RANGE.clone(),
        };

        command_buffer.pipeline_barrier(
            PipelineStage::TRANSFER .. PipelineStage::FRAGMENT_SHADER,
            m::Dependencies::empty(),
            &[image_barrier],
        );
    }
}
