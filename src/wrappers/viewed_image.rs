use std::cmp::max;
use ash::Device;
use ash::vk::{
    CommandBuffer, ComponentMapping, DescriptorPool, DescriptorSetLayout, Format, Image,
    ImageAspectFlags, ImageSubresourceRange, ImageUsageFlags, ImageView, ImageViewCreateFlags,
    ImageViewCreateInfo, ImageViewType, Sampler,
};
use byteorder::{LittleEndian, ReadBytesExt};
use gpu_allocator::vulkan::Allocator;
use crate::vulkan_helper::CallbackCollector;
use crate::wrappers::gpu_image::GpuImage;

#[derive(Debug)]
pub struct ViewedImage {
    image: GpuImage,
    image_view: ImageView,
}

impl ViewedImage {
    pub fn new(
        device: &Device,
        allocator: &mut Allocator,
        width: u32,
        height: u32,
        mip_levels: u32,
        size: u32,
        format: Format,
        long_live_staging: bool,
        dedicated_alloc: bool,
    ) -> Self {
        let image = GpuImage::new(
            device,
            allocator,
            width,
            height,
            mip_levels,
            size,
            format,
            ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::SAMPLED,
            long_live_staging,
            dedicated_alloc,
        );
        let image_view = unsafe {
            device
                .create_image_view(
                    &ImageViewCreateInfo::default()
                        .flags(ImageViewCreateFlags::empty())
                        .image(image.image())
                        .view_type(ImageViewType::TYPE_2D)
                        .format(format)
                        .components(ComponentMapping::default())
                        .subresource_range(
                            ImageSubresourceRange::default()
                                .aspect_mask(ImageAspectFlags::COLOR)
                                .base_mip_level(0)
                                .level_count(mip_levels)
                                .base_array_layer(0)
                                .layer_count(1),
                        ),
                    None,
                )
                .unwrap()
        };
        Self { image, image_view }
    }

    pub fn from_bc_bytes(
        device: &Device,
        cmd: CommandBuffer,
        allocator: &mut Allocator,
        on_finish: &mut CallbackCollector,
        mut data: &[u8],
        long_live_staging: bool,
        dedicated_alloc: bool,
    ) -> Self {
        let format = match data.read_u32::<LittleEndian>().unwrap() {
            0 => Format::BC7_SRGB_BLOCK,
            1 => Format::BC5_UNORM_BLOCK,
            2 => Format::BC4_UNORM_BLOCK,
            id => panic!("Unsupported bc format id: {:?}", id),
        };
        let width = data.read_u32::<LittleEndian>().unwrap();
        let height = data.read_u32::<LittleEndian>().unwrap();
        let depth = data.read_u32::<LittleEndian>().unwrap();
        assert_eq!(1, depth); //todo
        let layers = data.read_u32::<LittleEndian>().unwrap();
        assert_eq!(1, layers); //todo
        let mipmaps = data.read_u32::<LittleEndian>().unwrap();
        let mut image = Self::new(
            device,
            allocator,
            width,
            height,
            mipmaps,
            data.len() as u32,
            format,
            long_live_staging,
            dedicated_alloc,
        );
        let mut mipmap_offsets = Vec::with_capacity(mipmaps as usize);
        let mut offset = 0;
        let block_size = if format == Format::BC7_SRGB_BLOCK || format == Format::BC5_UNORM_BLOCK {
            16
        } else {
            8
        };
        for i in 0..mipmaps as usize {
            mipmap_offsets.push(offset);
            let block_w = max(1, width >> (i)).div_ceil(4);
            let block_h = max(1, height >> (i)).div_ceil(4);
            offset += block_w * block_h * block_size;
        }
        image.send_data_to_gpu(device, cmd, allocator, on_finish, data, &mipmap_offsets);
        image
    }

    pub fn from_mono_color(
        device: &Device,
        cmd: CommandBuffer,
        allocator: &mut Allocator,
        on_finish: &mut CallbackCollector,
        data: &[u8],
        format: Format,
    ) -> Self {
        let mut image = Self::new(
            device,
            allocator,
            1,
            1,
            1,
            data.len() as u32,
            format,
            false,
            false,
        );
        image.send_data_to_gpu(
            device,
            cmd,
            allocator,
            on_finish,
            bytemuck::cast_slice(&data),
            &[0],
        );
        image
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
        unsafe {
            device.destroy_image_view(self.image_view, None);
        }
        self.image.destroy(device, allocator);
    }

    pub fn image(&self) -> Image {
        self.image.image()
    }

    pub fn view(&self) -> ImageView {
        self.image_view
    }

    pub fn width(&self) -> u32 {
        self.image.width()
    }

    pub fn height(&self) -> u32 {
        self.image.height()
    }
}
