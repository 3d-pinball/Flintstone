use ash::Device;
use ash::vk::{Buffer, BufferUsageFlags, CommandBuffer, DeviceSize, IndexType};
use gpu_allocator::vulkan::Allocator;
use crate::vulkan_helper::CallbackCollector;
use crate::wrappers::gpu_buffer::GpuBuffer;

#[derive(Debug)]
pub struct VIBuffer {
    buffer: GpuBuffer,
    vertices_data_size: u32,
    indices_count: u32,
}

impl VIBuffer {
    pub fn new(
        device: &Device,
        cmd: CommandBuffer,
        allocator: &mut Allocator,
        on_finish: &mut CallbackCollector,
        vertices: &[u8],
        indices: &[u32],
        long_live_staging: bool,
        dedicated_alloc: bool,
    ) -> Self {
        let mut data = Vec::with_capacity(vertices.len() + indices.len() * 4);
        data.extend_from_slice(&vertices);
        data.extend_from_slice(bytemuck::cast_slice(indices));
        let mut buffer = GpuBuffer::new(
            device,
            allocator,
            data.len() as DeviceSize,
            BufferUsageFlags::VERTEX_BUFFER | BufferUsageFlags::INDEX_BUFFER,
            true,
            long_live_staging,
            dedicated_alloc,
        );
        buffer.send_data_to_gpu(device, cmd, allocator, on_finish, &data, 0);
        Self {
            buffer,
            vertices_data_size: vertices.len() as u32,
            indices_count: indices.len() as u32,
        }
    }

    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        self.buffer.destroy(device, allocator);
    }

    pub fn bind(&self, device:&Device,cmd: CommandBuffer) {
        unsafe {
            device.cmd_bind_vertex_buffers2(
                cmd,
                0,
                &[self.buffer.buffer()],
                &[0],
                Some(&[self.vertices_data_size as DeviceSize]),
                None,
            );
            device.cmd_bind_index_buffer(
                cmd,
                self.buffer.buffer(),
                self.vertices_data_size as DeviceSize,
                IndexType::UINT32,
            );
        }
    }
    pub fn buffer(&self) -> Buffer {
        self.buffer.buffer()
    }

    pub fn vertices_data_size(&self) -> u32 {
        self.vertices_data_size
    }

    pub fn indices_count(&self) -> u32 {
        self.indices_count
    }
}
