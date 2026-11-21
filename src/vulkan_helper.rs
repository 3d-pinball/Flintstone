use crate::wrappers::gpu_image::GpuImage;
use ash::vk::{
    ApplicationInfo, Bool32, ColorSpaceKHR, CommandBuffer, CommandBufferAllocateInfo,
    CommandBufferLevel, CommandPool, CommandPoolCreateFlags, CommandPoolCreateInfo,
    ComponentMapping, CompositeAlphaFlagsKHR, DebugUtilsMessageSeverityFlagsEXT,
    DebugUtilsMessageTypeFlagsEXT, DebugUtilsMessengerCallbackDataEXT,
    DebugUtilsMessengerCreateInfoEXT, DeviceCreateInfo, DeviceQueueCreateInfo, Extent2D, Fence,
    FenceCreateFlags, FenceCreateInfo, Format, Image, ImageAspectFlags, ImageSubresourceRange,
    ImageUsageFlags, ImageView, ImageViewCreateFlags, ImageViewCreateInfo, ImageViewType,
    InstanceCreateInfo, PhysicalDevice, PhysicalDeviceExtendedDynamicStateFeaturesEXT,
    PhysicalDeviceFeatures, PhysicalDeviceFeatures2, PhysicalDeviceProperties,
    PhysicalDeviceProperties2, PhysicalDeviceVulkan11Features, PhysicalDeviceVulkan12Features,
    PhysicalDeviceVulkan13Features, PresentModeKHR, Queue, QueueFlags, Semaphore,
    SemaphoreCreateFlags, SemaphoreCreateInfo, ShaderModule, ShaderModuleCreateInfo, SharingMode,
    SurfaceFormatKHR, SurfaceKHR, SwapchainCreateInfoKHR, SwapchainKHR,
};
use ash::{Device, Entry, khr, vk};
use glfw::PWindow;
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use gpu_allocator::{AllocationSizes, AllocatorDebugSettings};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::collections::HashMap;
use std::error::Error;
use std::ffi::{CStr, c_void};

pub fn create_vulkan(
    window: &PWindow,
    app_name: &CStr,
    app_version: (u32, u32, u32, u32),
) -> Result<(ash::Instance, SurfaceKHR, khr::surface::Instance), Box<dyn Error>> {
    let entry = Entry::linked();
    // dbg!(unsafe { entry.enumerate_instance_extension_properties(None)? });
    let display_handle = window.display_handle().map_err(|e| e.to_string())?.as_raw();
    let mut extensions = ash_window::enumerate_required_extensions(display_handle)?.to_vec();
    extensions.push(vk::EXT_DEBUG_UTILS_NAME.as_ptr());
    let app_info = ApplicationInfo::default()
        .application_name(app_name)
        .application_version(vk::make_api_version(
            app_version.0,
            app_version.1,
            app_version.2,
            app_version.3,
        ))
        .engine_name(c"Whelm")
        .engine_version(vk::make_api_version(
            app_version.0,
            app_version.1,
            app_version.2,
            app_version.3,
        ))
        .api_version(vk::API_VERSION_1_3);
    let layers = if cfg!(debug_assertions) {
        &[c"VK_LAYER_KHRONOS_validation".as_ptr()]
    } else {
        &[][..]
    };
    let vk_create_info = InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_extension_names(&extensions)
        .enabled_layer_names(layers);
    let vulkan = if cfg!(debug_assertions) {
        unsafe {
            entry.create_instance(
                &vk_create_info.push_next(
                    &mut DebugUtilsMessengerCreateInfoEXT::default()
                        .message_severity(
                            DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                                | DebugUtilsMessageSeverityFlagsEXT::INFO
                                | DebugUtilsMessageSeverityFlagsEXT::WARNING
                                | DebugUtilsMessageSeverityFlagsEXT::ERROR,
                        )
                        .message_type(
                            DebugUtilsMessageTypeFlagsEXT::GENERAL
                                | DebugUtilsMessageTypeFlagsEXT::VALIDATION
                                | DebugUtilsMessageTypeFlagsEXT::PERFORMANCE, // | DebugUtilsMessageTypeFlagsEXT::DEVICE_ADDRESS_BINDING,
                        )
                        .pfn_user_callback(Some(debug_utils_callback)),
                ),
                None,
            )?
        }
    } else {
        unsafe { entry.create_instance(&vk_create_info, None)? }
    };
    let surface = unsafe {
        ash_window::create_surface(
            &entry,
            &vulkan,
            display_handle,
            window.window_handle().unwrap().as_raw(),
            None,
        )?
    };
    let surface_loader = ash::khr::surface::Instance::new(&entry, &vulkan);
    Ok((vulkan, surface, surface_loader))
}

unsafe extern "system" fn debug_utils_callback(
    severity: DebugUtilsMessageSeverityFlagsEXT,
    types: DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut c_void,
) -> Bool32 {
    let message = unsafe { CStr::from_ptr((*callback_data).p_message) };
    println!("[{severity:?}] [{types:?}] {message:?}");
    vk::FALSE
}

pub fn choose_physical_device(
    vulkan: &ash::Instance,
) -> Result<(PhysicalDevice, PhysicalDeviceProperties), Box<dyn Error>> {
    let mut chosen = None;
    let mut props = PhysicalDeviceProperties2::default();
    for physical_device in unsafe { vulkan.enumerate_physical_devices()? } {
        unsafe { vulkan.get_physical_device_properties2(physical_device, &mut props) };
        let extensions = unsafe { vulkan.enumerate_device_extension_properties(physical_device)? };
        let features_vk10 = PhysicalDeviceFeatures::default();
        let mut features_vk11 = PhysicalDeviceVulkan11Features::default();
        let mut features_vk12 = PhysicalDeviceVulkan12Features::default();
        let mut features_vk13 = PhysicalDeviceVulkan13Features::default();
        let mut features_extended_dynamic_state =
            PhysicalDeviceExtendedDynamicStateFeaturesEXT::default();
        let mut features = PhysicalDeviceFeatures2::default()
            .features(features_vk10)
            .push_next(&mut features_vk11)
            .push_next(&mut features_vk12)
            .push_next(&mut features_vk13)
            .push_next(&mut features_extended_dynamic_state);
        unsafe { vulkan.get_physical_device_features2(physical_device, &mut features) };
        if props.properties.api_version < vk::API_VERSION_1_3
            || !extensions.iter().any(|e| unsafe {
                CStr::from_ptr(e.extension_name.as_ptr()) == vk::KHR_SWAPCHAIN_NAME
            })
            || !(features.features.sampler_anisotropy == vk::TRUE
                && features_vk11.shader_draw_parameters == vk::TRUE
                && features_vk12.buffer_device_address == vk::TRUE
                && features_vk13.dynamic_rendering == vk::TRUE
                && features_vk13.synchronization2 == vk::TRUE
                && features_vk13.inline_uniform_block == vk::TRUE
                && features_extended_dynamic_state.extended_dynamic_state == vk::TRUE)
        {
            continue;
        };
        chosen = Some(physical_device);
        println!(
            "Chosen physical device: {:?}",
            props
                .properties
                .device_name_as_c_str()
                .unwrap_or(c"Device name error"),
        );
        // dbg!("{:?}", props);
    }
    Ok((
        chosen.ok_or("Failed to found a suitable physical device")?,
        props.properties,
    ))
}

pub fn create_device(
    vulkan: &ash::Instance,
    physical_device: PhysicalDevice,
    surface_loader: &ash::khr::surface::Instance,
    surface: SurfaceKHR,
    needs_transfer_queue: bool,
    needs_compute_queue: bool,
) -> Result<
    (
        Device,
        (Queue, u32),
        (Queue, u32),
        Option<(Queue, u32)>,
        Option<(Queue, u32)>,
    ),
    Box<dyn Error>,
> {
    let queue_family_props =
        unsafe { vulkan.get_physical_device_queue_family_properties(physical_device) };
    dbg!(&queue_family_props);

    let mut graphics_index = None;
    let mut present_index = None;
    let mut transfer_index = None;
    let mut compute_index = None;
    let mut graphics_max_score = 0;
    let mut present_max_score = 0;
    let mut transfer_max_score = 0;
    let mut compute_max_score = 0;
    for prop in queue_family_props.iter().enumerate() {
        if prop.1.queue_count <= 0 {
            continue;
        }
        let present = unsafe {
            surface_loader.get_physical_device_surface_support(
                physical_device,
                prop.0 as u32,
                surface,
            )?
        };
        if prop.1.queue_flags.contains(QueueFlags::GRAPHICS) && graphics_index.is_none() {
            let mut graphics_score = 1;
            if present {
                graphics_score += 1;
            }
            if graphics_score > graphics_max_score {
                graphics_index = Some(prop.0);
                graphics_max_score = graphics_score;
                if present {
                    present_index = Some(prop.0);
                    present_max_score = 1;
                }
            }
        } else if present && present_max_score < 1 {
            present_index = Some(prop.0);
        }
        if prop.1.queue_flags.contains(QueueFlags::TRANSFER) {
            let mut transfer_score = 3;
            if prop.1.queue_flags.contains(QueueFlags::GRAPHICS) {
                transfer_score -= 1;
            }
            if prop.1.queue_flags.contains(QueueFlags::COMPUTE) {
                transfer_score -= 1;
            }
            if transfer_score > transfer_max_score {
                transfer_max_score = transfer_score;
                transfer_index = Some(prop.0);
            }
        }
        if prop.1.queue_flags.contains(QueueFlags::COMPUTE) {
            let mut compute_score = 2;
            if prop.1.queue_flags.contains(QueueFlags::GRAPHICS) {
                compute_score -= 1;
            }
            if compute_score > compute_max_score {
                compute_max_score = compute_score;
                compute_index = Some(prop.0);
            }
        }
    }
    let graphics_queue_family_index =
        graphics_index.expect("No graphics queue family found") as u32;
    let present_queue_family_index = present_index.expect("No present queue family found") as u32;
    let transfer_queue_family_index = if needs_transfer_queue {
        transfer_index.expect("No transfer queue family found") as u32
    } else {
        0
    };
    let compute_queue_family_index = if needs_compute_queue {
        compute_index.expect("No compute queue family found") as u32
    } else {
        0
    };

    let mut v = vec![graphics_queue_family_index, present_queue_family_index];
    if needs_transfer_queue {
        v.push(transfer_queue_family_index);
    }
    if needs_compute_queue {
        v.push(compute_queue_family_index);
    }
    v.sort_unstable();
    v.dedup();
    let mut queue_infos = Vec::new();
    v.iter().for_each(|index| {
        queue_infos.push(
            DeviceQueueCreateInfo::default()
                .queue_family_index(*index)
                .queue_priorities(&[1.0f32]),
        )
    });
    let device = unsafe {
        vulkan.create_device(
            physical_device,
            &DeviceCreateInfo::default()
                .queue_create_infos(&queue_infos)
                .enabled_extension_names(&[vk::KHR_SWAPCHAIN_NAME.as_ptr()])
                .push_next(
                    &mut PhysicalDeviceFeatures2::default()
                        .features(PhysicalDeviceFeatures::default().sampler_anisotropy(true))
                        .push_next(
                            &mut PhysicalDeviceVulkan11Features::default()
                                .shader_draw_parameters(true),
                        )
                        .push_next(
                            &mut PhysicalDeviceVulkan12Features::default()
                                .buffer_device_address(true),
                        )
                        .push_next(
                            &mut PhysicalDeviceVulkan13Features::default()
                                .dynamic_rendering(true)
                                .synchronization2(true)
                                .inline_uniform_block(true),
                        )
                        .push_next(
                            &mut PhysicalDeviceExtendedDynamicStateFeaturesEXT::default()
                                .extended_dynamic_state(true),
                        ),
                ),
            None,
        )
    }?;
    let mut map = HashMap::new();
    for index in v {
        map.insert(index, unsafe { device.get_device_queue(index, 0) });
    }

    Ok((
        device,
        (
            *map.get(&graphics_queue_family_index).unwrap(),
            graphics_queue_family_index,
        ),
        (
            *map.get(&present_queue_family_index).unwrap(),
            present_queue_family_index,
        ),
        if needs_transfer_queue {
            Some((
                *map.get(&transfer_queue_family_index).unwrap(),
                transfer_queue_family_index,
            ))
        } else {
            None
        },
        if needs_compute_queue {
            Some((
                *map.get(&compute_queue_family_index).unwrap(),
                compute_queue_family_index,
            ))
        } else {
            None
        },
    ))
}

pub fn create_swapchain(
    vulkan: &ash::Instance,
    physical_device: PhysicalDevice,
    device: &Device,
    surface_loader: &ash::khr::surface::Instance,
    surface: SurfaceKHR,
    window: &PWindow,
) -> Result<
    (
        SwapchainKHR,
        khr::swapchain::Device,
        Extent2D,
        SurfaceFormatKHR,
        Vec<Image>,
        Vec<ImageView>,
    ),
    Box<dyn Error>,
> {
    let surface_capabilities = unsafe {
        surface_loader.get_physical_device_surface_capabilities(physical_device, surface)
    }?;
    let surface_formats =
        unsafe { surface_loader.get_physical_device_surface_formats(physical_device, surface) }?;
    let surface_present_modes = unsafe {
        surface_loader.get_physical_device_surface_present_modes(physical_device, surface)
    }?;
    let mut min_image_count = u32::min(surface_capabilities.min_image_count + 1, 3);
    if surface_capabilities.max_image_count > 0 {
        min_image_count = u32::min(min_image_count, surface_capabilities.max_image_count);
    }
    let mut chosen_format = *surface_formats.get(0).unwrap();
    for format in surface_formats {
        if format.format == Format::B8G8R8A8_SRGB
            && format.color_space == ColorSpaceKHR::SRGB_NONLINEAR
        {
            chosen_format = format;
            break;
        }
    }
    let swapchain_extent = if surface_capabilities.current_extent.width != u32::MAX {
        surface_capabilities.current_extent
    } else {
        let (width, height) = window.get_framebuffer_size();
        Extent2D {
            width: u32::clamp(
                width as u32,
                surface_capabilities.min_image_extent.width,
                surface_capabilities.max_image_extent.width,
            ),
            height: u32::clamp(
                height as u32,
                surface_capabilities.min_image_extent.height,
                surface_capabilities.max_image_extent.height,
            ),
        }
    };
    let mut chosen_present_mode = *surface_present_modes.get(0).unwrap();
    let mut present_mode_max_score = 0;
    for mode in surface_present_modes {
        let score = match mode {
            PresentModeKHR::MAILBOX => 4,
            PresentModeKHR::FIFO => 3,
            PresentModeKHR::FIFO_RELAXED => 2,
            PresentModeKHR::IMMEDIATE => 1,
            _ => 0,
        };
        if score > present_mode_max_score {
            chosen_present_mode = mode;
            present_mode_max_score = score;
        }
    }
    let swapchain_device = khr::swapchain::Device::new(&vulkan, &device);
    let swapchain = unsafe {
        swapchain_device.create_swapchain(
            &SwapchainCreateInfoKHR::default()
                .surface(surface)
                .min_image_count(min_image_count)
                .image_format(chosen_format.format)
                .image_color_space(chosen_format.color_space)
                .image_extent(swapchain_extent)
                .image_array_layers(1)
                .image_usage(ImageUsageFlags::COLOR_ATTACHMENT)
                .image_sharing_mode(SharingMode::EXCLUSIVE)
                .pre_transform(surface_capabilities.current_transform)
                .composite_alpha(CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(chosen_present_mode)
                .clipped(true),
            None,
        )
    }?;
    let images = unsafe { swapchain_device.get_swapchain_images(swapchain) }?;
    let mut image_view_create_info = ImageViewCreateInfo::default()
        .flags(ImageViewCreateFlags::empty())
        .view_type(ImageViewType::TYPE_2D)
        .format(chosen_format.format)
        .components(ComponentMapping::default())
        .subresource_range(
            ImageSubresourceRange::default()
                .aspect_mask(ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1),
        );
    let mut image_views = Vec::new();
    for image in &images {
        image_view_create_info = image_view_create_info.image(*image);
        image_views.push(unsafe { device.create_image_view(&image_view_create_info, None)? });
    }
    Ok((
        swapchain,
        swapchain_device,
        swapchain_extent,
        chosen_format,
        images,
        image_views,
    ))
}

pub fn create_command_buffers(
    device: &Device,
    graphics_queue_family_index: u32,
    max_flight_frames: u32,
) -> Result<(CommandPool, Vec<CommandBuffer>, CommandBuffer), Box<dyn Error>> {
    let command_pool = unsafe {
        device.create_command_pool(
            &CommandPoolCreateInfo::default()
                .flags(
                    CommandPoolCreateFlags::TRANSIENT
                        | CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                )
                .queue_family_index(graphics_queue_family_index),
            None,
        )?
    };
    let mut command_buffers = unsafe {
        device.allocate_command_buffers(
            &CommandBufferAllocateInfo::default()
                .command_pool(command_pool)
                .level(CommandBufferLevel::PRIMARY)
                .command_buffer_count(max_flight_frames + 1),
        )?
    };
    let command_buffer_bootstrap = command_buffers.pop().unwrap();
    Ok((command_pool, command_buffers, command_buffer_bootstrap))
}

pub fn create_semaphores(
    device: &Device,
    swapchain_images_count: u32,
    max_flight_frames: u32,
) -> Result<(Vec<Semaphore>, Vec<Semaphore>, Vec<Fence>), Box<dyn Error>> {
    let mut semaphores_image_ready_for_render = Vec::with_capacity(max_flight_frames as usize);
    let mut semaphores_render_complete = Vec::with_capacity(swapchain_images_count as usize);
    let mut fences_draw_complete = Vec::with_capacity(max_flight_frames as usize);
    for _ in 0..max_flight_frames {
        semaphores_image_ready_for_render.push(unsafe {
            device.create_semaphore(
                &SemaphoreCreateInfo::default().flags(SemaphoreCreateFlags::empty()),
                None,
            )?
        });
        fences_draw_complete.push(unsafe {
            device.create_fence(
                &FenceCreateInfo::default().flags(FenceCreateFlags::SIGNALED),
                None,
            )?
        });
    }
    for _ in 0..swapchain_images_count {
        semaphores_render_complete.push(unsafe {
            device.create_semaphore(
                &SemaphoreCreateInfo::default().flags(SemaphoreCreateFlags::empty()),
                None,
            )?
        });
    }
    Ok((
        semaphores_image_ready_for_render,
        semaphores_render_complete,
        fences_draw_complete,
    ))
}

pub fn create_allocator(
    vulkan: ash::Instance,
    device: Device,
    physical_device: PhysicalDevice,
) -> Result<Allocator, Box<dyn Error>> {
    let mut debug_settings = AllocatorDebugSettings::default();
    if cfg!(debug_assertions) {
        debug_settings.log_memory_information = true;
        debug_settings.log_leaks_on_shutdown = true;
        debug_settings.store_stack_traces = true;
        debug_settings.log_allocations = true;
        debug_settings.log_frees = true;
        debug_settings.log_stack_traces = true;
    }
    Ok(Allocator::new(&AllocatorCreateDesc {
        instance: vulkan.clone(),
        device: device.clone(),
        physical_device,
        debug_settings,
        buffer_device_address: true,
        allocation_sizes: AllocationSizes::default(),
    })?)
}

pub fn create_depth_image(
    device: &Device,
    allocator: &mut Allocator,
    width: u32,
    height: u32,
) -> Result<(GpuImage, ImageView), Box<dyn Error>> {
    let image = GpuImage::new(
        device,
        allocator,
        width,
        height,
        1,
        width * height * 4,
        Format::D32_SFLOAT,
        ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        false,
        false,
    );
    let image_view = unsafe {
        device.create_image_view(
            &ImageViewCreateInfo::default()
                .flags(ImageViewCreateFlags::empty())
                .image(image.image())
                .view_type(ImageViewType::TYPE_2D)
                .format(Format::D32_SFLOAT)
                .components(ComponentMapping::default())
                .subresource_range(
                    ImageSubresourceRange::default()
                        .aspect_mask(ImageAspectFlags::DEPTH)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1),
                ),
            None,
        )?
    };
    Ok((image, image_view))
}

pub fn create_shader_module(device: &Device, code: &[u32]) -> ShaderModule {
    unsafe {
        device
            .create_shader_module(&ShaderModuleCreateInfo::default().code(code), None)
            .unwrap()
    }
}

pub struct CallbackCollector {
    callbacks: Vec<Box<dyn FnOnce(&Device, &mut Allocator)>>,
}

impl CallbackCollector {
    pub fn new() -> Self {
        Self {
            callbacks: Vec::new(),
        }
    }

    pub fn push(&mut self, callback: Box<dyn FnOnce(&Device, &mut Allocator)>) {
        self.callbacks.push(callback);
    }

    pub fn run_callbacks(self, device: &Device, allocator: &mut Allocator) {
        for callback in self.callbacks {
            callback(device, allocator);
        }
    }
}
