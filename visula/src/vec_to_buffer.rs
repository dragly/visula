use wgpu::util::DeviceExt;

pub fn vec_to_buffer<T>(
    device: &wgpu::Device,
    data: &[T],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    let contents = unsafe {
        // TODO consider if it is necessary to do to_vec here to obtain a copy
        &std::slice::from_raw_parts(data.as_ptr() as *const u8, std::mem::size_of_val(data))
            .to_vec()[..]
    };
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vec to buffer"),
        contents: bytemuck::cast_slice(contents),
        usage,
    })
}
