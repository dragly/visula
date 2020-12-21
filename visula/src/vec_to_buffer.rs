use wgpu::util::DeviceExt;

pub fn vec_to_buffer<T>(
    device: &wgpu::Device,
    data: &[T],
    usage: wgpu::BufferUsage,
) -> wgpu::Buffer {
    let contents = unsafe {
        // TODO consider if it is necessary to do to_vec here to obtain a copy
        &std::slice::from_raw_parts(
            data.as_ptr() as *const T as *const u8,
            data.len() * std::mem::size_of::<T>(),
        )
        .to_vec()[..]
    };
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vec to buffer"),
        contents: bytemuck::cast_slice(&contents),
        usage,
    })
}
