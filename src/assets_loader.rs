use crate::config_loader::ConfigLoader;
use crate::create_shader_module;
use crate::vulkan_helper::CallbackCollector;
use crate::wrappers::pods::{Pos3Norm3Tan4Tex2, PushDataModelTarget};
use crate::wrappers::vi_buffer::VIBuffer;
use crate::wrappers::viewed_image::ViewedImage;
use crate::wrappers::wrapped_uniform_buffer::WrappedUniformBuffer;
use ash::Device;
use ash::vk::{
    BlendFactor, BlendOp, BorderColor, ColorComponentFlags, CommandBuffer, CompareOp,
    CullModeFlags, DescriptorImageInfo, DescriptorPool, DescriptorPoolCreateFlags,
    DescriptorPoolCreateInfo, DescriptorPoolInlineUniformBlockCreateInfo, DescriptorPoolSize,
    DescriptorSet, DescriptorSetAllocateInfo, DescriptorSetLayout, DescriptorSetLayoutBinding,
    DescriptorSetLayoutCreateFlags, DescriptorSetLayoutCreateInfo, DescriptorType, DeviceSize,
    DynamicState, Filter, Format, FrontFace, GraphicsPipelineCreateInfo, ImageLayout, LogicOp,
    PhysicalDeviceProperties, Pipeline, PipelineCache, PipelineColorBlendAttachmentState,
    PipelineColorBlendStateCreateInfo, PipelineDepthStencilStateCreateInfo,
    PipelineDynamicStateCreateInfo, PipelineInputAssemblyStateCreateInfo, PipelineLayout,
    PipelineLayoutCreateInfo, PipelineMultisampleStateCreateInfo,
    PipelineRasterizationStateCreateInfo, PipelineRenderingCreateInfo,
    PipelineShaderStageCreateInfo, PipelineVertexInputStateCreateInfo,
    PipelineViewportStateCreateInfo, PolygonMode, PrimitiveTopology, PushConstantRange,
    SampleCountFlags, Sampler, SamplerAddressMode, SamplerCreateFlags, SamplerCreateInfo,
    SamplerMipmapMode, ShaderStageFlags, VertexInputAttributeDescription,
    VertexInputBindingDescription, VertexInputRate, WriteDescriptorSet,
    WriteDescriptorSetInlineUniformBlock,
};
use glam::Mat4;
use gpu_allocator::vulkan::Allocator;
use include_bytes_aligned::include_bytes_aligned;
use std::mem::offset_of;

pub struct AssetsLoader {
    pub desc_layout_vp: DescriptorSetLayout,
    pub desc_layout_material: DescriptorSetLayout,
    pub desc_layout_targets: DescriptorSetLayout,
    pub pipeline_layout: PipelineLayout,
    pub pipeline_pbr_pos3norm3tan4tex2: Pipeline,
    pub sampler: Sampler,
    pub desc_pool: DescriptorPool,
    pub uniform_bufs: Vec<WrappedUniformBuffer>,
    pub image_white: ViewedImage,
    pub image_black: ViewedImage,
    pub image_normal_default: ViewedImage,
    pub desc_material_default: DescriptorSet,
    pub vi_cube: VIBuffer,
    pub vi_square: VIBuffer,
    pub image_uv: ViewedImage,
    pub desc_material_ground: DescriptorSet,
}

impl AssetsLoader {
    pub fn new(
        config: &ConfigLoader,
        props: &PhysicalDeviceProperties,
        device: &Device,
        cmd: CommandBuffer,
        allocator: &mut Allocator,
        on_finish: &mut CallbackCollector,
        color_attachment_format: Format,
    ) -> Self {
        let desc_layout_vp = unsafe {
            device
                .create_descriptor_set_layout(
                    &DescriptorSetLayoutCreateInfo::default()
                        .flags(DescriptorSetLayoutCreateFlags::empty())
                        .bindings(&[DescriptorSetLayoutBinding::default()
                            .binding(0)
                            .descriptor_type(DescriptorType::UNIFORM_BUFFER)
                            .descriptor_count(1)
                            .stage_flags(ShaderStageFlags::VERTEX)]),
                    None,
                )
                .unwrap()
        };
        let desc_layout_material = unsafe {
            device
                .create_descriptor_set_layout(
                    &DescriptorSetLayoutCreateInfo::default()
                        .flags(DescriptorSetLayoutCreateFlags::empty())
                        .bindings(&[
                            DescriptorSetLayoutBinding::default()
                                .binding(0)
                                .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                                .descriptor_count(1)
                                .stage_flags(ShaderStageFlags::FRAGMENT),
                            DescriptorSetLayoutBinding::default()
                                .binding(1)
                                .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                                .descriptor_count(1)
                                .stage_flags(ShaderStageFlags::FRAGMENT),
                            DescriptorSetLayoutBinding::default()
                                .binding(2)
                                .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                                .descriptor_count(1)
                                .stage_flags(ShaderStageFlags::FRAGMENT),
                            DescriptorSetLayoutBinding::default()
                                .binding(3)
                                .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                                .descriptor_count(1)
                                .stage_flags(ShaderStageFlags::FRAGMENT),
                            DescriptorSetLayoutBinding::default()
                                .binding(4)
                                .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                                .descriptor_count(1)
                                .stage_flags(ShaderStageFlags::FRAGMENT),
                            DescriptorSetLayoutBinding::default()
                                .binding(5)
                                .descriptor_type(DescriptorType::INLINE_UNIFORM_BLOCK)
                                .descriptor_count(16)
                                .stage_flags(ShaderStageFlags::FRAGMENT),
                            DescriptorSetLayoutBinding::default()
                                .binding(6)
                                .descriptor_type(DescriptorType::INLINE_UNIFORM_BLOCK)
                                .descriptor_count(8)
                                .stage_flags(ShaderStageFlags::FRAGMENT),
                            DescriptorSetLayoutBinding::default()
                                .binding(7)
                                .descriptor_type(DescriptorType::INLINE_UNIFORM_BLOCK)
                                .descriptor_count(4)
                                .stage_flags(ShaderStageFlags::FRAGMENT),
                            DescriptorSetLayoutBinding::default()
                                .binding(8)
                                .descriptor_type(DescriptorType::INLINE_UNIFORM_BLOCK)
                                .descriptor_count(4)
                                .stage_flags(ShaderStageFlags::FRAGMENT),
                        ]),
                    None,
                )
                .unwrap()
        };
        let desc_layout_targets = unsafe {
            device
                .create_descriptor_set_layout(
                    &DescriptorSetLayoutCreateInfo::default()
                        .flags(DescriptorSetLayoutCreateFlags::empty())
                        .bindings(&[DescriptorSetLayoutBinding::default()
                            .binding(0)
                            .descriptor_type(DescriptorType::STORAGE_BUFFER)
                            .descriptor_count(1)
                            .stage_flags(ShaderStageFlags::VERTEX)]),
                    None,
                )
                .unwrap()
        };
        let module_pbr_pos3norm3tan4tex2 = create_shader_module!(&device, "pbr_pos3norm3tan4tex2");
        let pipeline_layout = unsafe {
            device
                .create_pipeline_layout(
                    &PipelineLayoutCreateInfo::default()
                        .set_layouts(&[desc_layout_vp, desc_layout_material, desc_layout_targets])
                        .push_constant_ranges(&[PushConstantRange::default()
                            .stage_flags(ShaderStageFlags::VERTEX)
                            .offset(0)
                            .size(size_of::<PushDataModelTarget>() as u32)]),
                    None,
                )
                .unwrap()
        };
        let pipelines = unsafe {
            device
                .create_graphics_pipelines(
                    PipelineCache::null(),
                    &[GraphicsPipelineCreateInfo::default()
                        .stages(&[
                            PipelineShaderStageCreateInfo::default()
                                .stage(ShaderStageFlags::VERTEX)
                                .module(module_pbr_pos3norm3tan4tex2)
                                .name(c"vertMain"),
                            PipelineShaderStageCreateInfo::default()
                                .stage(ShaderStageFlags::FRAGMENT)
                                .module(module_pbr_pos3norm3tan4tex2)
                                .name(c"fragMain"),
                        ])
                        .vertex_input_state(
                            &PipelineVertexInputStateCreateInfo::default()
                                .vertex_binding_descriptions(&[
                                    VertexInputBindingDescription::default()
                                        .binding(0)
                                        .stride(size_of::<Pos3Norm3Tan4Tex2>() as u32)
                                        .input_rate(VertexInputRate::VERTEX),
                                ])
                                .vertex_attribute_descriptions(&[
                                    VertexInputAttributeDescription::default()
                                        .location(0)
                                        .binding(0)
                                        .format(Format::R32G32B32_SFLOAT)
                                        .offset(offset_of!(Pos3Norm3Tan4Tex2, pos) as u32),
                                    VertexInputAttributeDescription::default()
                                        .location(1)
                                        .binding(0)
                                        .format(Format::R32G32B32_SFLOAT)
                                        .offset(offset_of!(Pos3Norm3Tan4Tex2, normal) as u32),
                                    VertexInputAttributeDescription::default()
                                        .location(2)
                                        .binding(0)
                                        .format(Format::R32G32B32A32_SFLOAT)
                                        .offset(offset_of!(Pos3Norm3Tan4Tex2, tangent) as u32),
                                    VertexInputAttributeDescription::default()
                                        .location(3)
                                        .binding(0)
                                        .format(Format::R32G32_SFLOAT)
                                        .offset(offset_of!(Pos3Norm3Tan4Tex2, tex) as u32),
                                ]),
                        )
                        .input_assembly_state(
                            &PipelineInputAssemblyStateCreateInfo::default()
                                .topology(PrimitiveTopology::TRIANGLE_LIST)
                                .primitive_restart_enable(false),
                        )
                        .viewport_state(
                            &PipelineViewportStateCreateInfo::default()
                                .viewport_count(1)
                                .scissor_count(1),
                        )
                        .rasterization_state(
                            &PipelineRasterizationStateCreateInfo::default()
                                .depth_clamp_enable(false)
                                .rasterizer_discard_enable(false)
                                .polygon_mode(PolygonMode::FILL)
                                .cull_mode(CullModeFlags::BACK)
                                .front_face(FrontFace::COUNTER_CLOCKWISE)
                                .depth_bias_enable(false)
                                .line_width(1.0),
                        )
                        .multisample_state(
                            &PipelineMultisampleStateCreateInfo::default()
                                .rasterization_samples(SampleCountFlags::TYPE_1)
                                .sample_shading_enable(false),
                        )
                        .depth_stencil_state(
                            &PipelineDepthStencilStateCreateInfo::default()
                                .depth_test_enable(true)
                                .depth_write_enable(true)
                                .depth_compare_op(CompareOp::LESS)
                                .depth_bounds_test_enable(false)
                                .stencil_test_enable(false),
                        )
                        .color_blend_state(
                            &PipelineColorBlendStateCreateInfo::default()
                                .logic_op_enable(false)
                                .logic_op(LogicOp::COPY)
                                .attachments(&[PipelineColorBlendAttachmentState::default()
                                    .blend_enable(false)
                                    .src_color_blend_factor(BlendFactor::SRC_ALPHA)
                                    .dst_color_blend_factor(BlendFactor::ONE_MINUS_SRC_ALPHA)
                                    .color_blend_op(BlendOp::ADD)
                                    .src_alpha_blend_factor(BlendFactor::ONE)
                                    .dst_alpha_blend_factor(BlendFactor::ONE_MINUS_SRC_ALPHA)
                                    .alpha_blend_op(BlendOp::ADD)
                                    .color_write_mask(ColorComponentFlags::RGBA)]),
                        )
                        .dynamic_state(
                            &PipelineDynamicStateCreateInfo::default()
                                .dynamic_states(&[DynamicState::VIEWPORT, DynamicState::SCISSOR]),
                        )
                        .layout(pipeline_layout)
                        .push_next(
                            &mut PipelineRenderingCreateInfo::default()
                                .color_attachment_formats(&[color_attachment_format])
                                .depth_attachment_format(Format::D32_SFLOAT),
                        )],
                    None,
                )
                .unwrap()
        };
        let pipeline_pbr_pos3norm3tan4tex2 = pipelines[0];
        unsafe {
            device.destroy_shader_module(module_pbr_pos3norm3tan4tex2, None);
        };
        let sampler = unsafe {
            device
                .create_sampler(
                    &SamplerCreateInfo::default()
                        .flags(SamplerCreateFlags::empty())
                        .mag_filter(Filter::LINEAR)
                        .min_filter(Filter::LINEAR)
                        .mipmap_mode(SamplerMipmapMode::LINEAR)
                        .address_mode_u(SamplerAddressMode::REPEAT)
                        .address_mode_v(SamplerAddressMode::REPEAT)
                        .address_mode_w(SamplerAddressMode::REPEAT)
                        .mip_lod_bias(0.0)
                        .anisotropy_enable(config.enable_sampler_anisotropy)
                        .max_anisotropy(props.limits.max_sampler_anisotropy)
                        .compare_enable(false)
                        .compare_op(CompareOp::ALWAYS)
                        .min_lod(0.0)
                        .max_lod(0.0)
                        .border_color(BorderColor::INT_OPAQUE_WHITE)
                        .unnormalized_coordinates(false),
                    None,
                )
                .unwrap()
        };
        let desc_pool = unsafe {
            device
                .create_descriptor_pool(
                    &DescriptorPoolCreateInfo::default()
                        .flags(DescriptorPoolCreateFlags::empty())
                        // .max_sets(render_preset.max_flight_frames + 2)
                        .max_sets(1024)
                        .pool_sizes(&[
                            DescriptorPoolSize::default()
                                .ty(DescriptorType::UNIFORM_BUFFER)
                                .descriptor_count(config.max_flight_frames),
                            DescriptorPoolSize::default()
                                .ty(DescriptorType::COMBINED_IMAGE_SAMPLER)
                                // .descriptor_count(2),
                                .descriptor_count(1024),
                            DescriptorPoolSize::default()
                                .ty(DescriptorType::INLINE_UNIFORM_BLOCK)
                                .descriptor_count(1024),
                            DescriptorPoolSize::default()
                                .ty(DescriptorType::STORAGE_BUFFER)
                                .descriptor_count(1024),
                        ])
                        .push_next(
                            &mut DescriptorPoolInlineUniformBlockCreateInfo::default()
                                .max_inline_uniform_block_bindings(1024),
                        ),
                    None,
                )
                .unwrap()
        };
        let mut uniform_bufs = Vec::with_capacity(config.max_flight_frames as usize);
        for _ in 0..config.max_flight_frames {
            uniform_bufs.push(WrappedUniformBuffer::new(
                device,
                allocator,
                size_of::<Mat4>() as DeviceSize,
                desc_pool,
                desc_layout_vp,
                0,
                0,
                false,
            ));
        }
        let image_white = ViewedImage::from_mono_color(
            &device,
            cmd,
            allocator,
            on_finish,
            &[255, 255, 255, 255],
            Format::R8G8B8A8_UNORM,
        );
        let image_black = ViewedImage::from_mono_color(
            &device,
            cmd,
            allocator,
            on_finish,
            &[0, 0, 0, 0],
            Format::R8G8B8A8_UNORM,
        );
        let image_normal_default = ViewedImage::from_mono_color(
            &device,
            cmd,
            allocator,
            on_finish,
            &[128, 128, 255, 255],
            Format::R8G8B8A8_UNORM,
        );
        let desc_material_default = unsafe {
            device
                .allocate_descriptor_sets(
                    &DescriptorSetAllocateInfo::default()
                        .descriptor_pool(desc_pool)
                        .set_layouts(&[desc_layout_material]),
                )
                .unwrap()[0]
        };
        unsafe {
            device.update_descriptor_sets(
                &[
                    WriteDescriptorSet::default()
                        .dst_set(desc_material_default)
                        .dst_binding(0)
                        .dst_array_element(0)
                        .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(&[DescriptorImageInfo::default()
                            .sampler(sampler)
                            .image_view(image_white.view())
                            .image_layout(ImageLayout::SHADER_READ_ONLY_OPTIMAL)]),
                    WriteDescriptorSet::default()
                        .dst_set(desc_material_default)
                        .dst_binding(1)
                        .dst_array_element(0)
                        .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(&[DescriptorImageInfo::default()
                            .sampler(sampler)
                            .image_view(image_white.view())
                            .image_layout(ImageLayout::SHADER_READ_ONLY_OPTIMAL)]),
                    WriteDescriptorSet::default()
                        .dst_set(desc_material_default)
                        .dst_binding(2)
                        .dst_array_element(0)
                        .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(&[DescriptorImageInfo::default()
                            .sampler(sampler)
                            .image_view(image_normal_default.view())
                            .image_layout(ImageLayout::SHADER_READ_ONLY_OPTIMAL)]),
                    WriteDescriptorSet::default()
                        .dst_set(desc_material_default)
                        .dst_binding(3)
                        .dst_array_element(0)
                        .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(&[DescriptorImageInfo::default()
                            .sampler(sampler)
                            .image_view(image_white.view())
                            .image_layout(ImageLayout::SHADER_READ_ONLY_OPTIMAL)]),
                    WriteDescriptorSet::default()
                        .dst_set(desc_material_default)
                        .dst_binding(4)
                        .dst_array_element(0)
                        .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(&[DescriptorImageInfo::default()
                            .sampler(sampler)
                            .image_view(image_black.view())
                            .image_layout(ImageLayout::SHADER_READ_ONLY_OPTIMAL)]),
                    WriteDescriptorSet::default()
                        .dst_set(desc_material_default)
                        .dst_binding(5)
                        .dst_array_element(0)
                        .descriptor_count(16)
                        .descriptor_type(DescriptorType::INLINE_UNIFORM_BLOCK)
                        .push_next(
                            &mut WriteDescriptorSetInlineUniformBlock::default()
                                .data(bytemuck::cast_slice(&[1.0f32, 1.0, 1.0, 1.0])),
                        ),
                    WriteDescriptorSet::default()
                        .dst_set(desc_material_default)
                        .dst_binding(6)
                        .dst_array_element(0)
                        .descriptor_count(8)
                        .descriptor_type(DescriptorType::INLINE_UNIFORM_BLOCK)
                        .push_next(
                            &mut WriteDescriptorSetInlineUniformBlock::default()
                                .data(bytemuck::cast_slice(&[1.0f32, 1.0])),
                        ),
                    WriteDescriptorSet::default()
                        .dst_set(desc_material_default)
                        .dst_binding(7)
                        .dst_array_element(0)
                        .descriptor_count(4)
                        .descriptor_type(DescriptorType::INLINE_UNIFORM_BLOCK)
                        .push_next(
                            &mut WriteDescriptorSetInlineUniformBlock::default()
                                .data(bytemuck::cast_slice(&[1.0f32])),
                        ),
                    WriteDescriptorSet::default()
                        .dst_set(desc_material_default)
                        .dst_binding(8)
                        .dst_array_element(0)
                        .descriptor_count(4)
                        .descriptor_type(DescriptorType::INLINE_UNIFORM_BLOCK)
                        .push_next(
                            &mut WriteDescriptorSetInlineUniformBlock::default()
                                .data(bytemuck::cast_slice(&[1.0f32])),
                        ),
                ],
                &[],
            );
        }
        let vertices = [
            Pos3Norm3Tan4Tex2::new(-0.5, -0.5, -0.5),
            Pos3Norm3Tan4Tex2::new(0.5, -0.5, -0.5),
            Pos3Norm3Tan4Tex2::new(-0.5, -0.5, 0.5),
            Pos3Norm3Tan4Tex2::new(0.5, -0.5, 0.5),
            Pos3Norm3Tan4Tex2::new(-0.5, 0.5, -0.5),
            Pos3Norm3Tan4Tex2::new(0.5, 0.5, -0.5),
            Pos3Norm3Tan4Tex2::new(-0.5, 0.5, 0.5),
            Pos3Norm3Tan4Tex2::new(0.5, 0.5, 0.5),
        ];
        let indices = [
            2, 0, 3, 0, 1, 3, 6, 2, 7, 2, 3, 7, 4, 0, 6, 0, 2, 6, 7, 3, 5, 3, 1, 5, 5, 1, 4, 1, 0,
            4, 4, 6, 5, 6, 7, 5,
        ];
        let vi_cube = VIBuffer::new(
            device,
            cmd,
            allocator,
            on_finish,
            bytemuck::cast_slice(&vertices),
            &indices,
            false,
            false,
        );
        let vertices = [
            Pos3Norm3Tan4Tex2::from_pos_tex(-0.5, 0.0, -0.5, 0.0, 0.0),
            Pos3Norm3Tan4Tex2::from_pos_tex(0.5, 0.0, -0.5, 1.0, 0.0),
            Pos3Norm3Tan4Tex2::from_pos_tex(-0.5, 0.0, 0.5, 0.0, 1.0),
            Pos3Norm3Tan4Tex2::from_pos_tex(0.5, 0.0, 0.5, 1.0, 1.0),
        ];
        let indices = [0, 2, 1, 2, 3, 1];
        let vi_square = VIBuffer::new(
            device,
            cmd,
            allocator,
            on_finish,
            bytemuck::cast_slice(&vertices),
            &indices,
            false,
            false,
        );
        let image_uv = ViewedImage::from_bc_bytes(
            &device,
            cmd,
            allocator,
            on_finish,
            include_bytes_aligned!(4, concat!(env!("OUT_DIR"), "/uv.bc7")),
            false,
            false,
        );
        let desc_material_ground = unsafe {
            device
                .allocate_descriptor_sets(
                    &DescriptorSetAllocateInfo::default()
                        .descriptor_pool(desc_pool)
                        .set_layouts(&[desc_layout_material]),
                )
                .unwrap()[0]
        };
        unsafe {
            device.update_descriptor_sets(
                &[
                    WriteDescriptorSet::default()
                        .dst_set(desc_material_ground)
                        .dst_binding(0)
                        .dst_array_element(0)
                        .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(&[DescriptorImageInfo::default()
                            .sampler(sampler)
                            .image_view(image_uv.view())
                            .image_layout(ImageLayout::SHADER_READ_ONLY_OPTIMAL)]),
                    WriteDescriptorSet::default()
                        .dst_set(desc_material_ground)
                        .dst_binding(1)
                        .dst_array_element(0)
                        .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(&[DescriptorImageInfo::default()
                            .sampler(sampler)
                            .image_view(image_white.view())
                            .image_layout(ImageLayout::SHADER_READ_ONLY_OPTIMAL)]),
                    WriteDescriptorSet::default()
                        .dst_set(desc_material_ground)
                        .dst_binding(2)
                        .dst_array_element(0)
                        .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(&[DescriptorImageInfo::default()
                            .sampler(sampler)
                            .image_view(image_normal_default.view())
                            .image_layout(ImageLayout::SHADER_READ_ONLY_OPTIMAL)]),
                    WriteDescriptorSet::default()
                        .dst_set(desc_material_ground)
                        .dst_binding(3)
                        .dst_array_element(0)
                        .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(&[DescriptorImageInfo::default()
                            .sampler(sampler)
                            .image_view(image_white.view())
                            .image_layout(ImageLayout::SHADER_READ_ONLY_OPTIMAL)]),
                    WriteDescriptorSet::default()
                        .dst_set(desc_material_ground)
                        .dst_binding(4)
                        .dst_array_element(0)
                        .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(&[DescriptorImageInfo::default()
                            .sampler(sampler)
                            .image_view(image_black.view())
                            .image_layout(ImageLayout::SHADER_READ_ONLY_OPTIMAL)]),
                    WriteDescriptorSet::default()
                        .dst_set(desc_material_ground)
                        .dst_binding(5)
                        .dst_array_element(0)
                        .descriptor_count(16)
                        .descriptor_type(DescriptorType::INLINE_UNIFORM_BLOCK)
                        .push_next(
                            &mut WriteDescriptorSetInlineUniformBlock::default()
                                .data(bytemuck::cast_slice(&[1.0f32, 1.0, 1.0, 1.0])),
                        ),
                    WriteDescriptorSet::default()
                        .dst_set(desc_material_ground)
                        .dst_binding(6)
                        .dst_array_element(0)
                        .descriptor_count(8)
                        .descriptor_type(DescriptorType::INLINE_UNIFORM_BLOCK)
                        .push_next(
                            &mut WriteDescriptorSetInlineUniformBlock::default()
                                .data(bytemuck::cast_slice(&[1.0f32, 1.0])),
                        ),
                    WriteDescriptorSet::default()
                        .dst_set(desc_material_ground)
                        .dst_binding(7)
                        .dst_array_element(0)
                        .descriptor_count(4)
                        .descriptor_type(DescriptorType::INLINE_UNIFORM_BLOCK)
                        .push_next(
                            &mut WriteDescriptorSetInlineUniformBlock::default()
                                .data(bytemuck::cast_slice(&[1.0f32])),
                        ),
                    WriteDescriptorSet::default()
                        .dst_set(desc_material_ground)
                        .dst_binding(8)
                        .dst_array_element(0)
                        .descriptor_count(4)
                        .descriptor_type(DescriptorType::INLINE_UNIFORM_BLOCK)
                        .push_next(
                            &mut WriteDescriptorSetInlineUniformBlock::default()
                                .data(bytemuck::cast_slice(&[1.0f32])),
                        ),
                ],
                &[],
            );
        }
        Self {
            desc_layout_vp,
            desc_layout_material,
            desc_layout_targets,
            pipeline_layout,
            pipeline_pbr_pos3norm3tan4tex2,
            sampler,
            desc_pool,
            uniform_bufs,
            image_white,
            image_black,
            image_normal_default,
            desc_material_default,
            vi_cube,
            vi_square,
            image_uv,
            desc_material_ground
        }
    }

    pub fn clean(&mut self, device: &Device, allocator: &mut Allocator) {
        self.image_white.destroy(device, allocator);
        self.image_black.destroy(device, allocator);
        self.image_normal_default.destroy(device, allocator);
        self.vi_cube.destroy(device, allocator);
        self.vi_square.destroy(device, allocator);
        self.image_uv.destroy(device, allocator);
        
        self.uniform_bufs
            .iter_mut()
            .for_each(|buf| buf.destroy(device, allocator));
        unsafe {
            device.destroy_descriptor_pool(self.desc_pool, None);
            device.destroy_sampler(self.sampler, None);
            device.destroy_pipeline(self.pipeline_pbr_pos3norm3tan4tex2, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_descriptor_set_layout(self.desc_layout_vp, None);
            device.destroy_descriptor_set_layout(self.desc_layout_material, None);
            device.destroy_descriptor_set_layout(self.desc_layout_targets, None);
        }
    }
}
