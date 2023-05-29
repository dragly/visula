pub trait InstanceBinding<'a> {
    fn handle(&self) -> uuid::Uuid;
    fn buffer(&'a self) -> &'a wgpu::Buffer;
    fn count(&self) -> u32;
    fn bind_group(&'a self) -> &'a wgpu::BindGroup;
}
