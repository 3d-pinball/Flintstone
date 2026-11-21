use crate::assets_loader::AssetsLoader;
use crate::config_loader::ConfigLoader;
use crate::vulkan_helper;
use crate::vulkan_helper::CallbackCollector;
use crate::wrappers::gpu_image::GpuImage;
use crate::wrappers::pods::PushDataModelTarget;
use ash::vk::{
    AccessFlags2, AttachmentLoadOp, AttachmentStoreOp, ClearColorValue, ClearDepthStencilValue,
    ClearValue, CommandBuffer, CommandBufferBeginInfo, CommandBufferSubmitInfo,
    CommandBufferUsageFlags, CommandPool, DependencyFlags, DependencyInfo, DeviceSize, Extent2D,
    Fence, Image, ImageAspectFlags, ImageLayout, ImageMemoryBarrier2, ImageSubresourceRange,
    ImageView, IndexType, Offset2D, PhysicalDevice, PhysicalDeviceProperties, PipelineBindPoint,
    PipelineStageFlags, PipelineStageFlags2, PresentInfoKHR, Queue, Rect2D,
    RenderingAttachmentInfo, RenderingInfo, Semaphore, SemaphoreCreateFlags, SemaphoreCreateInfo,
    ShaderStageFlags, SubmitInfo, SubmitInfo2, SurfaceFormatKHR, SurfaceKHR, SwapchainKHR,
    Viewport,
};
use ash::{Device, khr, vk};
use glam::camera::rh;
use glam::{Mat4, Vec3};
use glfw::{ClientApiHint, Glfw, GlfwReceiver, PWindow, WindowEvent, WindowHint, fail_on_errors};
use gpu_allocator::vulkan::Allocator;
use std::error::Error;
use std::mem::offset_of;

pub const MIN_WIDTH: u32 = 640;
pub const MIN_HEIGHT: u32 = 480;

pub struct RenderHandler {
    glfw: Glfw,
    window: PWindow,
    event_handle: GlfwReceiver<(f64, WindowEvent)>,
    vulkan: ash::Instance,
    surface: SurfaceKHR,
    surface_loader: khr::surface::Instance,
    phy_dev: PhysicalDevice,
    phy_dev_props: PhysicalDeviceProperties,
    device: Device,
    graphics_queue: Queue,
    present_queue: Queue,
    compute_queue: Queue,
    graphics_queue_family: u32,
    present_queue_family: u32,
    compute_queue_family: u32,
    swapchain: SwapchainKHR,
    swapchain_device: khr::swapchain::Device,
    swapchain_extent: Extent2D,
    surface_format: SurfaceFormatKHR,
    swapchain_images: Vec<Image>,
    swapchain_image_views: Vec<ImageView>,
    cmd_pool: CommandPool,
    cmd_bufs: Vec<CommandBuffer>,
    cmd_bootstrap: CommandBuffer,
    sems_ready_for_render: Vec<Semaphore>,
    sems_render_complete: Vec<Semaphore>,
    fences_draw_complete: Vec<Fence>,
    allocator: Option<Allocator>,
    depth_image: GpuImage,
    depth_image_view: ImageView,
    should_recreate_swapchain: bool,
    flight_frame_index: usize,
    assets_loader: AssetsLoader,
}

impl RenderHandler {
    pub fn create(config: &ConfigLoader) -> Result<Self, Box<dyn Error>> {
        let mut glfw = glfw::init(fail_on_errors!())?;
        glfw.window_hint(WindowHint::Visible(true));
        glfw.window_hint(WindowHint::ClientApi(ClientApiHint::NoApi));
        let (mut window, event_handle) = glfw
            .create_window(
                MIN_WIDTH,
                MIN_HEIGHT,
                "Iron Age",
                glfw::WindowMode::Windowed,
            )
            .expect("Failed to create GLFW window.");
        window.set_size_limits(Some(MIN_WIDTH), Some(MIN_HEIGHT), None, None);
        window.set_framebuffer_size_polling(true);
        window.set_key_polling(true);

        let (vulkan, surface, surface_loader) =
            vulkan_helper::create_vulkan(&window, c"Iron Age", crate::APP_VERSION)?;
        let (phy_dev, phy_dev_props) = vulkan_helper::choose_physical_device(&vulkan)?;
        let (
            device,
            (graphics_queue, graphics_queue_family),
            (present_queue, present_queue_family),
            _,
            compute,
        ) = vulkan_helper::create_device(&vulkan, phy_dev, &surface_loader, surface, false, true)?;
        let (compute_queue, compute_queue_family) = compute.unwrap();
        let (
            swapchain,
            swapchain_device,
            swapchain_extent,
            surface_format,
            swapchain_images,
            swapchain_image_views,
        ) = vulkan_helper::create_swapchain(
            &vulkan,
            phy_dev,
            &device,
            &surface_loader,
            surface,
            &window,
        )?;
        let (cmd_pool, cmd_bufs, cmd_bootstrap) = vulkan_helper::create_command_buffers(
            &device,
            graphics_queue_family,
            config.max_flight_frames,
        )?;
        let (sems_ready_for_render, sems_render_complete, fences_draw_complete) =
            vulkan_helper::create_semaphores(
                &device,
                swapchain_images.len() as u32,
                config.max_flight_frames,
            )?;
        let mut allocator =
            vulkan_helper::create_allocator(vulkan.clone(), device.clone(), phy_dev)?;
        let (depth_image, depth_image_view) = vulkan_helper::create_depth_image(
            &device,
            &mut allocator,
            swapchain_extent.width,
            swapchain_extent.height,
        )?;
        unsafe {
            device
                .begin_command_buffer(
                    cmd_bootstrap,
                    &CommandBufferBeginInfo::default()
                        .flags(CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .unwrap();
        }
        let mut on_finish = CallbackCollector::new();
        let assets_loader = AssetsLoader::new(
            config,
            &phy_dev_props,
            &device,
            cmd_bootstrap,
            &mut allocator,
            &mut on_finish,
            surface_format.format,
        );
        unsafe {
            device.end_command_buffer(cmd_bootstrap).unwrap();
            device
                .queue_submit2(
                    graphics_queue,
                    &[SubmitInfo2::default()
                        .wait_semaphore_infos(&[])
                        .command_buffer_infos(&[CommandBufferSubmitInfo::default()
                            .command_buffer(cmd_bootstrap)
                            .device_mask(0)])
                        .signal_semaphore_infos(&[])],
                    Fence::null(),
                )
                .unwrap();
            device.device_wait_idle().unwrap();
        }
        on_finish.run_callbacks(&device, &mut allocator);
        Ok(Self {
            glfw,
            window,
            event_handle,
            vulkan,
            surface,
            surface_loader,
            phy_dev,
            phy_dev_props,
            device,
            graphics_queue,
            present_queue,
            compute_queue,
            graphics_queue_family,
            present_queue_family,
            compute_queue_family,
            swapchain,
            swapchain_device,
            swapchain_extent,
            surface_format,
            swapchain_images,
            swapchain_image_views,
            cmd_pool,
            cmd_bufs,
            cmd_bootstrap,
            sems_ready_for_render,
            sems_render_complete,
            fences_draw_complete,
            allocator: Some(allocator),
            depth_image,
            depth_image_view,
            should_recreate_swapchain: false,
            flight_frame_index: 0,
            assets_loader,
        })
    }

    pub fn render_world(&mut self, cmd: CommandBuffer) {
        unsafe {
            self.device.cmd_bind_pipeline(
                cmd,
                PipelineBindPoint::GRAPHICS,
                self.assets_loader.pipeline_pbr_pos3norm3tan4tex2,
            );
            self.device.cmd_set_viewport(
                cmd,
                0,
                &[Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: self.swapchain_extent.width as f32,
                    height: self.swapchain_extent.height as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            );
            self.device.cmd_set_scissor(
                cmd,
                0,
                &[Rect2D {
                    offset: Offset2D { x: 0, y: 0 },
                    extent: self.swapchain_extent,
                }],
            );
        }
        let model = Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0));
        let view = rh::view::look_at_mat4(
            Vec3::new(0.0, 1.0, 2.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let proj = rh::proj::vulkan::perspective(
            45.0,
            self.swapchain_extent.width as f32 / self.swapchain_extent.height as f32,
            0.1,
            10.0,
        );
        let uniform_buf = &mut self.assets_loader.uniform_bufs[self.flight_frame_index];
        uniform_buf.send_data(bytemuck::cast_slice(&(proj * view).to_cols_array()), 0);
        unsafe {
            self.device.cmd_bind_descriptor_sets(
                cmd,
                PipelineBindPoint::GRAPHICS,
                self.assets_loader.pipeline_layout,
                0,
                &[uniform_buf.desc()],
                &[],
            );
        }

        unsafe {
            self.assets_loader.vi_square.bind(&self.device, cmd);
            self.device.cmd_bind_descriptor_sets(
                cmd,
                PipelineBindPoint::GRAPHICS,
                self.assets_loader.pipeline_layout,
                1,
                &[self.assets_loader.desc_material_ground],
                &[],
            );
            let model = model
                * Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0))
                * Mat4::from_scale(Vec3::splat(4.0));
            self.device.cmd_push_constants(
                cmd,
                self.assets_loader.pipeline_layout,
                ShaderStageFlags::VERTEX,
                offset_of!(PushDataModelTarget, model) as u32,
                bytemuck::cast_slice(&model.to_cols_array()),
            );
            self.device.cmd_draw_indexed(
                cmd,
                self.assets_loader.vi_square.indices_count(),
                1,
                0,
                0,
                0,
            );
        }
    }

    pub fn render(&mut self) {
        unsafe {
            self.device
                .wait_for_fences(
                    &[self.fences_draw_complete[self.flight_frame_index]],
                    true,
                    u64::MAX,
                )
                .unwrap();
        };
        let result = unsafe {
            self.swapchain_device.acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.sems_ready_for_render[self.flight_frame_index],
                Fence::null(),
            )
        };
        if result == Err(vk::Result::ERROR_OUT_OF_DATE_KHR) {
            if let Err(e) = self.recreate_swapchain() {
                panic!("Failed to recreate swapchain: {:?}", e);
            }
            return self.render();
        }
        unsafe {
            self.device
                .reset_fences(&[self.fences_draw_complete[self.flight_frame_index]])
                .unwrap();
        }

        let (image_index, suboptimal) = result.unwrap();
        if suboptimal {
            self.should_recreate_swapchain = true;
        }
        let cmd = self.cmd_bufs[self.flight_frame_index];
        unsafe {
            self.device
                .begin_command_buffer(
                    cmd,
                    &CommandBufferBeginInfo::default()
                        .flags(CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .unwrap();
            self.device.cmd_pipeline_barrier2(
                self.cmd_bufs[self.flight_frame_index],
                &DependencyInfo::default()
                    .dependency_flags(DependencyFlags::empty())
                    .image_memory_barriers(&[
                        ImageMemoryBarrier2::default()
                            .src_stage_mask(PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                            .src_access_mask(AccessFlags2::empty())
                            .dst_stage_mask(PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                            .dst_access_mask(AccessFlags2::COLOR_ATTACHMENT_WRITE)
                            .old_layout(ImageLayout::UNDEFINED)
                            .new_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                            .src_queue_family_index(
                                if self.graphics_queue_family == self.present_queue_family {
                                    vk::QUEUE_FAMILY_IGNORED
                                } else {
                                    self.present_queue_family
                                },
                            )
                            .dst_queue_family_index(
                                if self.graphics_queue_family == self.present_queue_family {
                                    vk::QUEUE_FAMILY_IGNORED
                                } else {
                                    self.graphics_queue_family
                                },
                            )
                            .image(self.swapchain_images[image_index as usize])
                            .subresource_range(
                                ImageSubresourceRange::default()
                                    .aspect_mask(ImageAspectFlags::COLOR)
                                    .base_mip_level(0)
                                    .level_count(1)
                                    .base_array_layer(0)
                                    .layer_count(1),
                            ),
                        ImageMemoryBarrier2::default()
                            .src_stage_mask(
                                PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                                    | PipelineStageFlags2::LATE_FRAGMENT_TESTS,
                            )
                            .src_access_mask(AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE)
                            .dst_stage_mask(
                                PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                                    | PipelineStageFlags2::LATE_FRAGMENT_TESTS,
                            )
                            .dst_access_mask(AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE)
                            .old_layout(ImageLayout::UNDEFINED)
                            .new_layout(ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
                            .src_queue_family_index(
                                if self.graphics_queue_family == self.present_queue_family {
                                    vk::QUEUE_FAMILY_IGNORED
                                } else {
                                    self.present_queue_family
                                },
                            )
                            .dst_queue_family_index(
                                if self.graphics_queue_family == self.present_queue_family {
                                    vk::QUEUE_FAMILY_IGNORED
                                } else {
                                    self.graphics_queue_family
                                },
                            )
                            .image(self.depth_image.image())
                            .subresource_range(
                                ImageSubresourceRange::default()
                                    .aspect_mask(ImageAspectFlags::DEPTH)
                                    .base_mip_level(0)
                                    .level_count(1)
                                    .base_array_layer(0)
                                    .layer_count(1),
                            ),
                    ]),
            );
            self.device.cmd_begin_rendering(
                cmd,
                &RenderingInfo::default()
                    .render_area(Rect2D {
                        offset: Offset2D { x: 0, y: 0 },
                        extent: self.swapchain_extent,
                    })
                    .layer_count(1)
                    .color_attachments(&[RenderingAttachmentInfo::default()
                        .image_view(self.swapchain_image_views[image_index as usize])
                        .image_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .load_op(AttachmentLoadOp::CLEAR)
                        .store_op(AttachmentStoreOp::STORE)
                        .clear_value(ClearValue {
                            color: ClearColorValue {
                                float32: [0.0, 0.0, 0.0, 1.0],
                            },
                        })])
                    .depth_attachment(
                        &RenderingAttachmentInfo::default()
                            .image_view(self.depth_image_view)
                            .image_layout(ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
                            .load_op(AttachmentLoadOp::CLEAR)
                            .store_op(AttachmentStoreOp::DONT_CARE)
                            .clear_value(ClearValue {
                                depth_stencil: ClearDepthStencilValue {
                                    depth: 1.0,
                                    stencil: 0,
                                },
                            }),
                    ),
            );
        }

        self.render_world(cmd);

        unsafe {
            self.device.cmd_end_rendering(cmd);
            self.device.cmd_pipeline_barrier2(
                self.cmd_bufs[self.flight_frame_index],
                &DependencyInfo::default()
                    .dependency_flags(DependencyFlags::empty())
                    .image_memory_barriers(&[ImageMemoryBarrier2::default()
                        .src_stage_mask(PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                        .src_access_mask(AccessFlags2::COLOR_ATTACHMENT_WRITE)
                        .dst_stage_mask(PipelineStageFlags2::BOTTOM_OF_PIPE)
                        .dst_access_mask(AccessFlags2::empty())
                        .old_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .new_layout(ImageLayout::PRESENT_SRC_KHR)
                        .src_queue_family_index(
                            if self.graphics_queue_family == self.present_queue_family {
                                vk::QUEUE_FAMILY_IGNORED
                            } else {
                                self.graphics_queue_family
                            },
                        )
                        .dst_queue_family_index(
                            if self.graphics_queue_family == self.present_queue_family {
                                vk::QUEUE_FAMILY_IGNORED
                            } else {
                                self.present_queue_family
                            },
                        )
                        .image(self.swapchain_images[image_index as usize])
                        .subresource_range(
                            ImageSubresourceRange::default()
                                .aspect_mask(ImageAspectFlags::COLOR)
                                .base_mip_level(0)
                                .level_count(1)
                                .base_array_layer(0)
                                .layer_count(1),
                        )]),
            );
            self.device.end_command_buffer(cmd).unwrap();
            self.device
                .queue_submit(
                    self.graphics_queue,
                    &[SubmitInfo::default()
                        .wait_semaphores(&[self.sems_ready_for_render[self.flight_frame_index]])
                        .wait_dst_stage_mask(&[PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                        .command_buffers(&[cmd])
                        .signal_semaphores(&[self.sems_render_complete[image_index as usize]])],
                    self.fences_draw_complete[self.flight_frame_index],
                )
                .unwrap();
        };
        let result = unsafe {
            self.swapchain_device.queue_present(
                self.present_queue,
                &PresentInfoKHR::default()
                    .wait_semaphores(&[self.sems_render_complete[image_index as usize]])
                    .swapchains(&[self.swapchain])
                    .image_indices(&[image_index]),
            )
        };
        self.flight_frame_index = (self.flight_frame_index + 1) % self.cmd_bufs.len();
        if result == Err(vk::Result::ERROR_OUT_OF_DATE_KHR)
            || result.unwrap()
            || self.should_recreate_swapchain
        {
            if let Err(e) = self.recreate_swapchain() {
                panic!("Failed to recreate swapchain: {:?}", e);
            }
        }
    }

    fn recreate_swapchain(&mut self) -> Result<(), Box<dyn Error>> {
        self.should_recreate_swapchain = false;
        println!("Recreate swapchain");
        unsafe {
            self.device.device_wait_idle()?;
            for image_view in &self.swapchain_image_views {
                self.device.destroy_image_view(*image_view, None);
            }
            self.swapchain_device
                .destroy_swapchain(self.swapchain, None);
        }
        (
            self.swapchain,
            self.swapchain_device,
            self.swapchain_extent,
            self.surface_format,
            self.swapchain_images,
            self.swapchain_image_views,
        ) = vulkan_helper::create_swapchain(
            &self.vulkan,
            self.phy_dev,
            &self.device,
            &self.surface_loader,
            self.surface,
            &self.window,
        )?;
        let i = self.swapchain_images.len() as i32 - self.sems_render_complete.len() as i32;
        if i > 0 {
            for _ in 0..i {
                self.sems_render_complete.push(unsafe {
                    self.device.create_semaphore(
                        &SemaphoreCreateInfo::default().flags(SemaphoreCreateFlags::empty()),
                        None,
                    )
                }?);
            }
        } else if i < 0 {
            for _ in 0..(-i) {
                unsafe {
                    self.device
                        .destroy_semaphore(self.sems_render_complete.pop().unwrap(), None)
                };
            }
        }
        self.depth_image
            .destroy(&self.device, self.allocator.as_mut().unwrap());
        unsafe {
            self.device.destroy_image_view(self.depth_image_view, None);
        }
        let (depth_image, depth_image_view) = vulkan_helper::create_depth_image(
            &self.device,
            self.allocator.as_mut().unwrap(),
            self.swapchain_extent.width,
            self.swapchain_extent.height,
        )?;
        self.depth_image = depth_image;
        self.depth_image_view = depth_image_view;
        Ok(())
    }

    pub fn clean(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
        }
        self.assets_loader
            .clean(&self.device, self.allocator.as_mut().unwrap());
        unsafe {
            self.device.destroy_image_view(self.depth_image_view, None);
        }
        self.depth_image
            .destroy(&self.device, self.allocator.as_mut().unwrap());
        drop(self.allocator.take());
        unsafe {
            for e in &self.fences_draw_complete {
                self.device.destroy_fence(*e, None);
            }
            for e in &self.sems_render_complete {
                self.device.destroy_semaphore(*e, None);
            }
            for e in &self.sems_ready_for_render {
                self.device.destroy_semaphore(*e, None);
            }
            self.device.destroy_command_pool(self.cmd_pool, None);
            for image_view in &self.swapchain_image_views {
                self.device.destroy_image_view(*image_view, None);
            }
            self.swapchain_device
                .destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.vulkan.destroy_instance(None);
        }
    }

    pub fn should_close(&self) -> bool {
        self.window.should_close()
    }

    pub fn pull_events(&mut self) {
        self.glfw.poll_events();
        for (event_time, event) in glfw::flush_messages(&self.event_handle) {
            dbg!((event_time, &event));
            match event {
                WindowEvent::FramebufferSize(_width, _height) => {
                    self.should_recreate_swapchain = true;
                }
                _ => {}
            }
        }
    }
}
