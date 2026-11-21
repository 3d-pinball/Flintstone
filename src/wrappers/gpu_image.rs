use ash::vk::{
    AccessFlags2, BindImageMemoryInfo, BufferImageCopy2, BufferUsageFlags, CommandBuffer,
    CopyBufferToImageInfo2, DependencyFlags, DependencyInfo, DeviceSize, Extent3D, Format, Image,
    ImageAspectFlags, ImageCreateFlags, ImageCreateInfo, ImageLayout, ImageMemoryBarrier2,
    ImageMemoryRequirementsInfo2, ImageSubresourceLayers, ImageSubresourceRange, ImageTiling,
    ImageType, ImageUsageFlags, MemoryRequirements2, Offset3D, PipelineStageFlags2,
    SampleCountFlags, SharingMode,
};
use ash::{Device, vk};
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator};
use crate::vulkan_helper::CallbackCollector;
use crate::wrappers::allocated_buffer::AllocatedBuffer;

#[derive(Debug)]
pub struct GpuImage {
    image: Image,
    allocation: Option<Allocation>,
    staging: Option<AllocatedBuffer>,
    width: u32,
    height: u32,
    size: u32,
}

impl GpuImage {
    pub fn new(
        device: &Device,
        allocator: &mut Allocator,
        width: u32,
        height: u32,
        mip_levels: u32,
        size: u32,
        format: Format,
        usage: ImageUsageFlags,
        long_live_staging: bool,
        dedicated_alloc: bool,
    ) -> Self {
        let image = unsafe {
            device.create_image(
                &ImageCreateInfo::default()
                    .flags(ImageCreateFlags::empty())
                    .image_type(ImageType::TYPE_2D)
                    .format(format)
                    .extent(Extent3D::default().width(width).height(height).depth(1))
                    .mip_levels(mip_levels)
                    .array_layers(1)
                    .samples(SampleCountFlags::TYPE_1)
                    .tiling(ImageTiling::OPTIMAL)
                    .usage(usage)
                    .sharing_mode(SharingMode::EXCLUSIVE)
                    .initial_layout(ImageLayout::UNDEFINED),
                None,
            )
        }
        .unwrap();
        let mut requirement = MemoryRequirements2::default();
        unsafe {
            device.get_image_memory_requirements2(
                &ImageMemoryRequirementsInfo2::default().image(image),
                &mut requirement,
            )
        };
        let allocation = allocator
            .allocate(&AllocationCreateDesc {
                name: "",
                requirements: requirement.memory_requirements,
                location: MemoryLocation::GpuOnly,
                linear: false,
                allocation_scheme: if dedicated_alloc {
                    AllocationScheme::DedicatedImage(image)
                } else {
                    AllocationScheme::GpuAllocatorManaged
                },
            })
            .unwrap();
        unsafe {
            device
                .bind_image_memory2(&[BindImageMemoryInfo::default()
                    .image(image)
                    .memory(allocation.memory())
                    .memory_offset(allocation.offset())])
                .unwrap();
        }
        let staging = if long_live_staging {
            Some(AllocatedBuffer::new(
                device,
                allocator,
                size as DeviceSize,
                BufferUsageFlags::TRANSFER_SRC,
                MemoryLocation::CpuToGpu,
                false,
                dedicated_alloc,
            ))
        } else {
            None
        };
        Self {
            image,
            allocation: Some(allocation),
            staging,
            width,
            height,
            size,
        }
    }

    pub fn send_data_to_gpu(
        &mut self,
        device: &Device,
        cmd: CommandBuffer,
        allocator: &mut Allocator,
        on_finish: &mut CallbackCollector,
        data: &[u8],
        mipmap_offsets: &[u32],
    ) {
        let mut temp = None;
        unsafe {
            device.cmd_pipeline_barrier2(
                cmd,
                &DependencyInfo::default()
                    .dependency_flags(DependencyFlags::empty())
                    .image_memory_barriers(&[ImageMemoryBarrier2::default()
                        .src_stage_mask(PipelineStageFlags2::NONE)
                        .src_access_mask(AccessFlags2::empty())
                        .dst_stage_mask(PipelineStageFlags2::TRANSFER)
                        .dst_access_mask(AccessFlags2::TRANSFER_WRITE)
                        .old_layout(ImageLayout::UNDEFINED)
                        .new_layout(ImageLayout::TRANSFER_DST_OPTIMAL)
                        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .image(self.image)
                        .subresource_range(
                            ImageSubresourceRange::default()
                                .aspect_mask(ImageAspectFlags::COLOR)
                                .base_mip_level(0)
                                .level_count(mipmap_offsets.len() as u32)
                                .base_array_layer(0)
                                .layer_count(1),
                        )]),
            );
        }
        let staging = if self.staging.is_some() {
            self.staging.as_mut().unwrap()
        } else {
            temp = Some(AllocatedBuffer::new(
                device,
                allocator,
                self.size as DeviceSize,
                BufferUsageFlags::TRANSFER_SRC,
                MemoryLocation::CpuToGpu,
                false,
                false,
            ));
            temp.as_mut().unwrap()
        };
        staging.send_data(data, 0);
        unsafe {
            device.cmd_copy_buffer_to_image2(
                cmd,
                &CopyBufferToImageInfo2::default()
                    .src_buffer(staging.buffer())
                    .dst_image(self.image)
                    .dst_image_layout(ImageLayout::TRANSFER_DST_OPTIMAL)
                    .regions(
                        mipmap_offsets
                            .iter()
                            .enumerate()
                            .map(|(i, &offset)| {
                                BufferImageCopy2::default()
                                    .buffer_offset(offset as DeviceSize)
                                    .buffer_row_length(0)
                                    .buffer_image_height(0)
                                    .image_subresource(
                                        ImageSubresourceLayers::default()
                                            .aspect_mask(ImageAspectFlags::COLOR)
                                            .mip_level(i as u32)
                                            .base_array_layer(0)
                                            .layer_count(1),
                                    )
                                    .image_offset(Offset3D { x: 0, y: 0, z: 0 })
                                    .image_extent(Extent3D {
                                        width: self.width >> i,
                                        height: self.height >> i,
                                        depth: 1,
                                    })
                            })
                            .collect::<Vec<_>>()
                            .as_slice(),
                    ),
            );
            device.cmd_pipeline_barrier2(
                cmd,
                &DependencyInfo::default()
                    .dependency_flags(DependencyFlags::empty())
                    .image_memory_barriers(&[ImageMemoryBarrier2::default()
                        .src_stage_mask(PipelineStageFlags2::TRANSFER)
                        .src_access_mask(AccessFlags2::TRANSFER_WRITE)
                        .dst_stage_mask(PipelineStageFlags2::FRAGMENT_SHADER)
                        .dst_access_mask(AccessFlags2::SHADER_READ)
                        .old_layout(ImageLayout::TRANSFER_DST_OPTIMAL)
                        .new_layout(ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .image(self.image)
                        .subresource_range(
                            ImageSubresourceRange::default()
                                .aspect_mask(ImageAspectFlags::COLOR)
                                .base_mip_level(0)
                                .level_count(mipmap_offsets.len() as u32)
                                .base_array_layer(0)
                                .layer_count(1),
                        )]),
            );
        }
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
        unsafe {
            device.destroy_image(self.image, None);
        }
        allocator.free(self.allocation.take().unwrap()).unwrap();
    }

    pub fn image(&self) -> Image {
        self.image
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}
