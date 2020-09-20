pub fn vec_to_buffer<T>(
    device: &wgpu::Device,
    data: &[T],
    usage: wgpu::BufferUsage,
) -> wgpu::Buffer {
    device.create_buffer_with_data(
        unsafe {
            // TODO consider if it is necessary to do to_vec here to obtain a copy
            &std::slice::from_raw_parts(
                data.as_ptr() as *const T as *const u8,
                data.len() * std::mem::size_of::<T>(),
            )
            .to_vec()[..]
        },
        usage,
    )
}
