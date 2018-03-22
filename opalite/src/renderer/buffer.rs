use std::{ marker::PhantomData, mem, ops::Drop, sync::{ Arc, Mutex } };
use failure::Error;
use hal;
use hal::{ buffer, memory as m, pso };
use hal::{ Backend, Device };

#[derive(Fail, Debug)]
pub enum BufferError {
    #[fail(display = "Cannot find valid memory type")]
    UploadType,
    #[fail(display = "Cannot create a buffer from ZST")]
    ZST,
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
        if D::STRIDE == 0 {
            Err(BufferError::ZST)?
        }

        let device = device_arc.lock().unwrap();

        let buffer_len = len * D::STRIDE;
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

    pub fn write(&mut self, data: &[D]) -> Result<(), Error> {
        assert!(self.len >= data.len() as u64);

        let device = self.device.lock().unwrap();
        let mut writer = device.acquire_mapping_writer::<D>(&self.memory, 0..data.len() as u64 * D::STRIDE)?;
        writer.copy_from_slice(data);
        device.release_mapping_writer(writer);

        Ok(())
    }

    pub fn descriptor_set<'a>(&'a self, binding: u32, array_offset: usize, descriptor_set: &'a B::DescriptorSet) -> pso::DescriptorSetWrite<'a, B, Option<pso::Descriptor<'a, B>>> {
        pso::DescriptorSetWrite {
            set: descriptor_set,
            binding,
            array_offset,
            descriptors: Some(pso::Descriptor::Buffer(self.buffer(), Some(0)..Some(D::STRIDE))),
        }
    }

    pub fn buffer(&self) -> &B::Buffer {
        &self.buffer.as_ref().unwrap()
    }

    pub fn len(&self) -> u32 {
        self.len as u32
    }

    pub fn usage(&self) -> buffer::Usage {
        self.usage
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
    const STRIDE: u64;

    fn len(&self) -> u64 { 1 }

    fn buffer_len(&self) -> u64 { self.len() * Self::STRIDE }

    fn desc() -> Vec<pso::AttributeDesc>;
}

impl BufferData for u32 {
    const STRIDE: u64 = mem::size_of::<Self>() as u64;

    fn desc() -> Vec<pso::AttributeDesc> {
        Vec::new()
    }
}
