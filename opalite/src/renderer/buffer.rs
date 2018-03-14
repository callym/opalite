use std::{ marker::PhantomData, mem, ops::Drop, sync::{ Arc, Mutex } };
use failure::Error;
use hal;
use hal::{ buffer, command, device as d, format as f, image as i, memory as m, pass, pso, pool };
use hal::{ Backend, Instance, PhysicalDevice, Surface, Swapchain };
use hal::{
    Adapter,
    DescriptorPool,
    Device,
    FrameSync,
    Primitive,
    Backbuffer,
    SwapchainConfig,
};
use hal::format::{ AsFormat, ChannelType, Rgba8Srgb as ColorFormat, Swizzle };
use hal::pass::Subpass;
use hal::pso::{ PipelineStage, ShaderStageFlags, Specialization };
use hal::queue::Submission;

#[derive(Fail, Debug)]
pub enum BufferError {
    #[fail(display = "Cannot get window size.")]
    UploadType,
}

pub struct Buffer<D: BufferData, B: Backend> {
    len: u64,
    memory: B::Memory,
    usage: buffer::Usage,
    buffer: Option<B::Buffer>,
    // drop after the above
    device: Arc<Mutex<B::Device>>,
    _phantom: PhantomData<D>,
}

impl<D: BufferData, B: Backend> Buffer<D, B> {
    pub fn new(device_arc: Arc<Mutex<B::Device>>, len: u64, usage: buffer::Usage, memory_types: &[hal::MemoryType]) -> Result<Self, Error> {
        let device = device_arc.lock().unwrap();

        let buffer_len = len * D::Stride;
        let buffer_unbound = device.create_buffer(buffer_len, usage)?;
        let buffer_req = device.get_buffer_requirements(&buffer_unbound);
        let upload_type = memory_types.iter().enumerate()
            .position(|(id, mem_type)| {
                buffer_req.type_mask & (1 << id) != 0 &&
                mem_type.properties.contains(m::Properties::CPU_VISIBLE)
            }).ok_or(BufferError::UploadType)?;

        let buffer_memory = device.allocate_memory(upload_type.into(), buffer_req.size)?;
        let buffer = device.bind_buffer_memory(&buffer_memory, 0, buffer_unbound)?;

        Ok(Self {
            len,
            memory: buffer_memory,
            usage,
            buffer: Some(buffer),
            device: device_arc.clone(),
            _phantom: PhantomData,
        })
    }

    pub fn write(&mut self, data: D) -> Result<(), Error> {
        assert!(self.len >= data.len());

        let device = self.device.lock().unwrap();
        let mut writer = device.acquire_mapping_writer::<D>(&self.memory, 0..data.buffer_len())?;
        writer.copy_from_slice(&[data]);
        device.release_mapping_writer(writer);

        Ok(())
    }

    pub fn buffer(&self) -> &B::Buffer {
        &self.buffer.as_ref().unwrap()
    }
}

impl<D: BufferData, B: Backend> Drop for Buffer<D, B> {
    fn drop(&mut self) {
        let device = self.device.lock().unwrap();
        let buffer = self.buffer.take().unwrap();
        device.destroy_buffer(buffer);
    }
}

pub trait BufferData: Copy {
    const Stride: u64 = mem::size_of::<Self>() as u64;

    fn len(&self) -> u64 { 1 }

    fn buffer_len(&self) -> u64 { self.len() * Self::Stride }

    fn desc() -> Vec<pso::AttributeDesc>;
}

impl<'a, D> BufferData for &'a [D] where D: BufferData {
    default fn len(&self) -> u64 {
        self.len() as u64
    }

    default fn desc() -> Vec<pso::AttributeDesc> {
        D::desc()
    }
}
