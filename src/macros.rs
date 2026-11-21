#[macro_export]
macro_rules! create_shader_module {
    ($device:expr, $name:expr) => {
        $crate::vulkan_helper::create_shader_module(
            $device,
            $crate::__private::bytemuck::cast_slice(
                $crate::__private::include_bytes_aligned::include_bytes_aligned!(
                    4,
                    concat!(env!("OUT_DIR"), "/", $name, ".spv")
                ),
            ),
        )
    };
}
