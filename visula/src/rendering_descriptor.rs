use wgpu::{Device, TextureFormat};

use crate::camera::Camera;

pub struct RenderingDescriptor<'a> {
    pub device: &'a Device,
    pub format: &'a TextureFormat,
    pub camera: &'a Camera,
    pub sample_count: u32,
}
