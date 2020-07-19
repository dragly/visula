pub fn vec_to_buffer<T>(
    device: &wgpu::Device,
    data: &Vec<T>,
    usage: wgpu::BufferUsage,
) -> wgpu::Buffer {
    device.create_buffer_with_data(
        unsafe {
            std::slice::from_raw_parts(
                data.as_ptr() as *const T as *const u8,
                data.len() * std::mem::size_of::<T>(),
            )
            .clone()
        },
        usage,
    )
}

