use crate::vulkan_helper::CallbackCollector;
use crate::wrappers::viewed_image::ViewedImage;
use ash::Device;
use ash::vk::{
    CommandBuffer, ComponentMapping, DescriptorImageInfo, DescriptorPool, DescriptorSet,
    DescriptorSetAllocateInfo, DescriptorSetLayout, DescriptorType, Format, Image,
    ImageAspectFlags, ImageLayout, ImageSubresourceRange, ImageUsageFlags, ImageView,
    ImageViewCreateFlags, ImageViewCreateInfo, ImageViewType, Sampler, WriteDescriptorSet,
};
use byteorder::{LittleEndian, ReadBytesExt};
use gpu_allocator::vulkan::Allocator;
use std::cmp::max;

#[derive(Debug)]
pub struct WrappedImage {
    image: ViewedImage,
    desc: DescriptorSet,
}

impl WrappedImage {
    pub fn new(
        device: &Device,
        allocator: &mut Allocator,
        width: u32,
        height: u32,
        mip_levels: u32,
        size: u32,
        format: Format,
        desc_pool: DescriptorPool,
        desc_layout: DescriptorSetLayout,
        binding: u32,
        index: u32,
        sampler: Sampler,
        long_live_staging: bool,
        dedicated_alloc: bool,
    ) -> Self {
        let image = ViewedImage::new(
            device,
            allocator,
            width,
            height,
            mip_levels,
            size,
            format,
            long_live_staging,
            dedicated_alloc,
        );
        Self::from_viewed_image(
            device,
            image,
            desc_pool,
            desc_layout,
            binding,
            index,
            sampler,
        )
    }

    pub fn from_viewed_image(
        device: &Device,
        image: ViewedImage,
        desc_pool: DescriptorPool,
        desc_layout: DescriptorSetLayout,
        binding: u32,
        index: u32,
        sampler: Sampler,
    ) -> Self {
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
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&[DescriptorImageInfo::default()
                        .sampler(sampler)
                        .image_view(image.view())
                        .image_layout(ImageLayout::SHADER_READ_ONLY_OPTIMAL)])],
                &[],
            );
        }
        Self { image, desc }
    }

    pub fn from_bc_bytes(
        device: &Device,
        cmd: CommandBuffer,
        allocator: &mut Allocator,
        on_finish: &mut CallbackCollector,
        data: &[u8],
        desc_pool: DescriptorPool,
        desc_layout: DescriptorSetLayout,
        binding: u32,
        index: u32,
        sampler: Sampler,
        long_live_staging: bool,
        dedicated_alloc: bool,
    ) -> Self {
        let image = ViewedImage::from_bc_bytes(
            device,
            cmd,
            allocator,
            on_finish,
            data,
            long_live_staging,
            dedicated_alloc,
        );
        Self::from_viewed_image(
            device,
            image,
            desc_pool,
            desc_layout,
            binding,
            index,
            sampler,
        )
    }

    pub fn from_mono_color(
        device: &Device,
        cmd: CommandBuffer,
        allocator: &mut Allocator,
        on_finish: &mut CallbackCollector,
        data: &[u8],
        format: Format,
        desc_pool: DescriptorPool,
        desc_layout: DescriptorSetLayout,
        binding: u32,
        index: u32,
        sampler: Sampler,
    ) -> Self {
        let image = ViewedImage::from_mono_color(device, cmd, allocator, on_finish, data, format);
        Self::from_viewed_image(
            device,
            image,
            desc_pool,
            desc_layout,
            binding,
            index,
            sampler,
        )
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
        self.image
            .send_data_to_gpu(device, cmd, allocator, on_finish, data, mipmap_offsets);
    }

    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        self.image.destroy(device, allocator);
    }

    pub fn image(&self) -> Image {
        self.image.image()
    }

    pub fn view(&self) -> ImageView {
        self.image.view()
    }

    pub fn width(&self) -> u32 {
        self.image.width()
    }

    pub fn height(&self) -> u32 {
        self.image.height()
    }

    pub fn desc(&self) -> DescriptorSet {
        self.desc
    }
}
