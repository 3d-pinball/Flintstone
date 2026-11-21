use crate::wrappers::allocated_buffer::AllocatedBuffer;
use ash::Device;
use ash::vk::{
    Buffer, BufferUsageFlags, DescriptorBufferInfo, DescriptorPool, DescriptorSet,
    DescriptorSetAllocateInfo, DescriptorSetLayout, DescriptorType, DeviceSize, WriteDescriptorSet,
};
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::Allocator;

#[derive(Debug)]
pub struct WrappedUniformBuffer {
    buffer: AllocatedBuffer,
    desc: DescriptorSet,
}

impl WrappedUniformBuffer {
    pub fn new(
        device: &Device,
        allocator: &mut Allocator,
        size: DeviceSize,
        desc_pool: DescriptorPool,
        desc_layout: DescriptorSetLayout,
        binding: u32,
        index: u32,
        dedicated_alloc: bool,
    ) -> Self {
        let buffer = AllocatedBuffer::new(
            device,
            allocator,
            size,
            BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            false,
            dedicated_alloc,
        );
        let desc = unsafe {
            device
                .allocate_descriptor_sets(
                    &DescriptorSetAllocateInfo::default()
                        .descriptor_pool(desc_pool)
                        .set_layouts(&[desc_layout]),
                )
                .unwrap()[0]
        };
        unsafe {
            device.update_descriptor_sets(
                &[WriteDescriptorSet::default()
                    .dst_set(desc)
                    .dst_binding(binding)
                    .dst_array_element(index)
                    .descriptor_type(DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(&[DescriptorBufferInfo::default()
                        .buffer(buffer.buffer())
                        .offset(0)
                        .range(size)])],
                &[],
            );
        }
        Self { buffer, desc }
    }

    pub fn send_data(&mut self, data: &[u8], dst_offset: usize) {
        self.buffer.send_data(data, dst_offset);
    }

    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        self.buffer.destroy(device, allocator);
    }

    pub fn buffer(&self) -> Buffer {
        self.buffer.buffer()
    }

    pub fn desc(&self) -> DescriptorSet {
        self.desc
    }
}
