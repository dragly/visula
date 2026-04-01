use wgpu::{Device, TextureFormat};

use crate::camera::Camera;
use crate::light::DirectionalLight;

pub struct RenderingDescriptor<'a> {
    pub device: &'a Device,
    pub format: &'a TextureFormat,
    pub camera: &'a Camera,
    pub light: &'a DirectionalLight,
    pub sample_count: u32,
}
