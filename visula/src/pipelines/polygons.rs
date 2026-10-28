use crate::pipelines::quad::{QuadPipeline, QuadPipelineDescriptor};
use crate::rendering_descriptor::RenderingDescriptor;
use crate::simulation::RenderData;
use crate::Renderable;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec4};
use std::mem::size_of;
use visula_core::Expression;
use visula_derive::Delegate;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct PolygonVertex {
    pub position: [f32; 2],
}

pub struct Polygons(QuadPipeline);

#[derive(Delegate)]
pub struct PolygonDelegate {
    pub color: Expression,
    pub transform: Expression,
}

impl Default for PolygonDelegate {
    fn default() -> Self {
        PolygonDelegate {
            color: Vec4::new(1.0, 1.0, 1.0, 1.0).into(),
            transform: Mat4::IDENTITY.into(),
        }
    }
}

impl Polygons {
    pub fn new(
        rendering_descriptor: &RenderingDescriptor,
        delegate: &PolygonDelegate,
        vertices: &[PolygonVertex],
        indices: &[u32],
    ) -> Result<Self, visula_core::ShaderError> {
        Ok(Polygons(QuadPipeline::new(
            rendering_descriptor,
            &QuadPipelineDescriptor {
                label: "polygons",
                shader_source: include_str!("../shaders/polygon.wgsl"),
                shader_variable_name: "polygon",
                fragment_shader_variable_name: None,
                shadow_shader_source: None,
                vertex_data: bytemuck::cast_slice(vertices),
                vertex_stride: size_of::<PolygonVertex>(),
                vertex_format: wgpu::VertexFormat::Float32x2,
                index_data: bytemuck::cast_slice(indices),
                index_format: wgpu::IndexFormat::Uint32,
                bind_group_layouts: &[],
                depth_write_enabled: true,
                depth_bias: wgpu::DepthBiasState::default(),
            },
            delegate,
            None,
        )?))
    }

    pub fn update_geometry(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[PolygonVertex],
        indices: &[u32],
    ) {
        self.0.update_geometry(
            device,
            queue,
            bytemuck::cast_slice(vertices),
            bytemuck::cast_slice(indices),
        );
    }
}

impl Renderable for Polygons {
    fn render(&self, render_data: &mut RenderData) {
        self.0.render(render_data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;
    use crate::light::DirectionalLight;
    use visula_core::InstanceBuffer;

    const TRIANGLE: [PolygonVertex; 3] = [
        PolygonVertex {
            position: [0.0, 0.0],
        },
        PolygonVertex {
            position: [1.0, 0.0],
        },
        PolygonVertex {
            position: [0.0, 1.0],
        },
    ];

    struct Gpu {
        device: wgpu::Device,
        queue: wgpu::Queue,
        camera: Camera,
        light: DirectionalLight,
    }

    impl Gpu {
        fn new() -> Self {
            let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
            let camera = Camera::new(&device);
            let light = DirectionalLight::new(&device);
            Gpu {
                device,
                queue,
                camera,
                light,
            }
        }

        fn polygons(
            &self,
            delegate: &PolygonDelegate,
            vertices: &[PolygonVertex],
            indices: &[u32],
        ) -> Polygons {
            let rendering_descriptor = RenderingDescriptor {
                device: &self.device,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                camera: &self.camera,
                light: &self.light,
                sample_count: 1,
            };
            Polygons::new(&rendering_descriptor, delegate, vertices, indices).unwrap()
        }
    }

    #[test]
    fn transform() {
        let delegate = PolygonDelegate {
            transform: Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2).into(),
            ..Default::default()
        };
        Gpu::new().polygons(&delegate, &TRIANGLE, &[0, 1, 2]);
    }

    #[test]
    fn instanced_transform() {
        let gpu = Gpu::new();
        let transforms = InstanceBuffer::<Mat4>::new(&gpu.device);
        transforms.update(
            &gpu.device,
            &gpu.queue,
            &[Mat4::IDENTITY, Mat4::from_rotation_z(1.0)],
        );
        let delegate = PolygonDelegate {
            transform: transforms.instance(),
            ..Default::default()
        };
        gpu.polygons(&delegate, &TRIANGLE, &[0, 1, 2]);
    }

    #[test]
    fn update_geometry() {
        let gpu = Gpu::new();
        let mut polygons = gpu.polygons(&PolygonDelegate::default(), &[], &[]);
        let two_triangles = [TRIANGLE, TRIANGLE].concat();
        polygons.update_geometry(&gpu.device, &gpu.queue, &TRIANGLE, &[0, 1, 2]);
        polygons.update_geometry(&gpu.device, &gpu.queue, &two_triangles, &[0, 1, 2, 3, 4, 5]);
        polygons.update_geometry(&gpu.device, &gpu.queue, &TRIANGLE, &[0, 1, 2]);
        polygons.update_geometry(&gpu.device, &gpu.queue, &[], &[]);
        gpu.queue.submit([]);
    }
}
