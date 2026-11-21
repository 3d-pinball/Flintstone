use ash::Device;
use ash::vk::{
    Buffer, BufferCopy2, BufferUsageFlags, CommandBuffer, CopyBufferInfo2, DeviceSize,
};
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::Allocator;
use crate::vulkan_helper::CallbackCollector;
use crate::wrappers::allocated_buffer::AllocatedBuffer;

#[derive(Debug)]
pub struct GpuBuffer {
    buffer: AllocatedBuffer,
    staging: Option<AllocatedBuffer>,
    size: DeviceSize,
    usage: BufferUsageFlags,
    linear: bool,
}

impl GpuBuffer {
    pub fn new(
        device: &Device,
        allocator: &mut Allocator,
        size: DeviceSize,
        usage: BufferUsageFlags,
        linear: bool,
        long_live_staging: bool,
        dedicated_alloc: bool,
    ) -> Self {
        let buffer = AllocatedBuffer::new(
            device,
            allocator,
            size,
            usage | BufferUsageFlags::TRANSFER_DST,
            MemoryLocation::GpuOnly,
            linear,
            dedicated_alloc,
        );
        let staging = if long_live_staging {
            Some(AllocatedBuffer::new(
                device,
                allocator,
                size,
                BufferUsageFlags::TRANSFER_SRC,
                MemoryLocation::CpuToGpu,
                linear,
                dedicated_alloc,
            ))
        } else {
            None
        };
        GpuBuffer {
            buffer,
            staging,
            size,
            usage,
            linear,
        }
    }

    pub fn send_data_to_gpu(
        &mut self,
        device: &Device,
        cmd: CommandBuffer,
        allocator: &mut Allocator,
        on_finish: &mut CallbackCollector,
        data: &[u8],
        dst_offset: usize,
    ) {
        let mut temp = None;
        let staging = if self.staging.is_some() {
            self.staging.as_mut().unwrap()
        } else {
            temp = Some(AllocatedBuffer::new(
                device,
                allocator,
                self.size,
                self.usage | BufferUsageFlags::TRANSFER_SRC,
                MemoryLocation::CpuToGpu,
                self.linear,
                false,
            ));
            temp.as_mut().unwrap()
        };
        staging.send_data(data, dst_offset);
        unsafe {
            device.cmd_copy_buffer2(
                cmd,
                &CopyBufferInfo2::default()
                    .src_buffer(staging.buffer())
                    .dst_buffer(self.buffer.buffer())
                    .regions(&[BufferCopy2::default()
                        .src_offset(dst_offset as DeviceSize)
                        .dst_offset(dst_offset as DeviceSize)
                        .size(data.len() as DeviceSize)]),
            )
        };
        if let Some(mut temp) = temp {
            on_finish.push(Box::new(
                move |device: &Device, allocator: &mut Allocator| {
                    temp.destroy(device, allocator);
                },
            ));
        }
    }

    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        if let Some(staging) = self.staging.as_mut() {
            staging.destroy(device, allocator);
        }
        self.buffer.destroy(device, allocator);
    }

    pub fn buffer(&self) -> Buffer {
        self.buffer.buffer()
    }

    pub fn size(&self) -> DeviceSize {
        self.size
    }
}
