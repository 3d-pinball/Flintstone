use ash::Device;
use ash::vk::{
    BindBufferMemoryInfo, Buffer, BufferCreateInfo, BufferMemoryRequirementsInfo2,
    BufferUsageFlags, DeviceSize, MemoryRequirements2, SharingMode,
};
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator};
use std::ptr;

#[derive(Debug)]
pub struct AllocatedBuffer {
    buffer: Buffer,
    allocation: Option<Allocation>,
    location: MemoryLocation,
}

impl AllocatedBuffer {
    pub fn new(
        device: &Device,
        allocator: &mut Allocator,
        size: DeviceSize,
        usage: BufferUsageFlags,
        location: MemoryLocation,
        linear: bool,
        dedicated_alloc: bool,
    ) -> AllocatedBuffer {
        let buffer = unsafe {
            device
                .create_buffer(
                    &BufferCreateInfo::default()
                        .size(size)
                        .usage(usage)
                        .sharing_mode(SharingMode::EXCLUSIVE),
                    None,
                )
                .unwrap()
        };
        let mut requirement = MemoryRequirements2::default();
        unsafe {
            device.get_buffer_memory_requirements2(
                &BufferMemoryRequirementsInfo2::default().buffer(buffer),
                &mut requirement,
            )
        }
        let allocation = allocator
            .allocate(&AllocationCreateDesc {
                name: "",
                requirements: requirement.memory_requirements,
                location,
                linear,
                allocation_scheme: if dedicated_alloc {
                    AllocationScheme::DedicatedBuffer(buffer)
                } else {
                    AllocationScheme::GpuAllocatorManaged
                },
            })
            .unwrap();
        unsafe {
            device
                .bind_buffer_memory2(&[BindBufferMemoryInfo::default()
                    .buffer(buffer)
                    .memory(allocation.memory())
                    .memory_offset(allocation.offset())])
                .unwrap();
        }
        AllocatedBuffer {
            buffer,
            allocation: Some(allocation),
            location,
        }
    }

    pub fn send_data(&mut self, data: &[u8], dst_offset: usize) {
        debug_assert!(dst_offset % 4 == 0);
        match self.location {
            MemoryLocation::CpuToGpu => {
                let mut p_dst = self
                    .allocation
                    .as_ref()
                    .unwrap()
                    .mapped_ptr()
                    .unwrap()
                    .as_ptr() as *mut u8;
                unsafe {
                    if dst_offset > 0 {
                        p_dst = p_dst.add(dst_offset);
                    }
                    ptr::copy_nonoverlapping(data.as_ptr(), p_dst, data.len());
                }
            }
            MemoryLocation::GpuOnly => {
                panic!("Can't write to GpuOnly memory")
            }
            MemoryLocation::GpuToCpu => {
                panic!("Can't write to GpuToCpu memory")
            }
            MemoryLocation::Unknown => {
                panic!("Can't write to Unknown memory")
            }
        }
    }

    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        unsafe {
            device.destroy_buffer(self.buffer, None);
        }
        allocator.free(self.allocation.take().unwrap()).unwrap();
    }

    pub fn buffer(&self) -> Buffer {
        self.buffer
    }
}
