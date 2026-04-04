use bytemuck::{Pod, Zeroable};
use glam::{Quat, Vec3};

use visula::{
    primitives::generate_torus, CylinderGeometry, CylinderMaterial, Cylinders, Expression,
    InstanceBuffer, InstanceDeviceExt, MeshGeometry, MeshMaterial, MeshPipeline, RenderData,
    Renderable, RenderingControls, ShadowRenderData, SphereGeometry, SphereMaterial,
    SpherePrimitive, Spheres,
};
use visula_derive::Instance;
use wgpu::util::DeviceExt;

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct CylinderData {
    start: [f32; 3],
    start_radius: f32,
    end: [f32; 3],
    end_radius: f32,
    color: [f32; 3],
    _pad: f32,
}

struct Simulation {
    cylinder: Cylinders,
    _cyl_buffer: InstanceBuffer<CylinderData>,
    torus_mesh: MeshPipeline,
    soma_spheres: Spheres,
    _soma_buffer: InstanceBuffer<SpherePrimitive>,
    rendering_controls: RenderingControls,
}

impl Simulation {
    fn new(app: &mut visula::Application) -> Self {
        let device = &app.device;

        // Cylinder
        let cyl_buffer: InstanceBuffer<CylinderData> = device.create_instance_buffer();
        let cyl_data = vec![CylinderData {
            start: [0.0, 0.0, 0.0],
            start_radius: 0.2,
            end: [0.0, 3.0, 0.0],
            end_radius: 0.2,
            color: [0.8, 0.2, 0.2],
            _pad: 0.0,
        }];
        cyl_buffer.update(device, &app.queue, &cyl_data);

        let cyl_instance = cyl_buffer.instance();
        let cylinder = Cylinders::new(
            &app.rendering_descriptor(),
            &CylinderGeometry {
                start: cyl_instance.start,
                end: cyl_instance.end,
                start_radius: cyl_instance.start_radius,
                end_radius: cyl_instance.end_radius,
                color: cyl_instance.color,
            },
            &CylinderMaterial {
                color: Expression::InputColor.toon_lit(),
            },
        )
        .unwrap();

        // Torus as mesh
        let torus_color = [50, 150, 75, 255];
        let (torus_verts, torus_indices) = generate_torus(3.0, 0.3, 48, 24, torus_color);
        let mut torus_mesh = MeshPipeline::new(
            &app.rendering_descriptor(),
            &MeshGeometry {
                position: Vec3::new(0.0, -1.0, 0.0).into(),
                rotation: Quat::IDENTITY.into(),
                scale: Vec3::ONE.into(),
            },
            &MeshMaterial {
                color: Expression::InputColor.lit(),
            },
        )
        .unwrap();
        torus_mesh.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Torus vertex buffer"),
            contents: bytemuck::cast_slice(&torus_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        torus_mesh.index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Torus index buffer"),
            contents: bytemuck::cast_slice(&torus_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        torus_mesh.vertex_count = torus_indices.len();

        // Sphere primitives (ray-traced) with .toon_lit()
        let soma_data = vec![
            SpherePrimitive {
                position: [-4.0, 1.0, 0.0],
                radius: 1.0,
                color: [1.0, 0.39, 0.20],
                padding: 0.0,
            },
            SpherePrimitive {
                position: [4.0, 1.0, 0.0],
                radius: 1.0,
                color: [0.39, 0.78, 0.20],
                padding: 0.0,
            },
        ];
        let soma_buffer: InstanceBuffer<SpherePrimitive> = device.create_instance_buffer();
        soma_buffer.update(device, &app.queue, &soma_data);

        let soma_instance = soma_buffer.instance();
        let soma_spheres = Spheres::new(
            &app.rendering_descriptor(),
            &SphereGeometry {
                position: soma_instance.position,
                radius: soma_instance.radius,
                color: soma_instance.color,
            },
            &SphereMaterial {
                color: Expression::InputColor.toon_lit(),
            },
        )
        .unwrap();

        app.post_processor.config.outline = visula::post_process::config::OutlineConfig {
            enabled: true,
            color: [0.0, 0.0, 0.0],
            thickness: 6.0,
            depth_threshold: 0.1,
        };

        Simulation {
            cylinder,
            _cyl_buffer: cyl_buffer,
            torus_mesh,
            soma_spheres,
            _soma_buffer: soma_buffer,
            rendering_controls: RenderingControls::new(),
        }
    }
}

impl visula::Simulation for Simulation {
    type Error = ();

    fn update(&mut self, application: &mut visula::Application) {
        self.rendering_controls.update(application);
    }

    fn render(&mut self, data: &mut RenderData) {
        self.cylinder.render(data);
        self.torus_mesh.render(data);
        self.soma_spheres.render(data);
    }

    fn render_shadow(&mut self, data: &mut ShadowRenderData) {
        self.cylinder.render_shadow(data);
        self.soma_spheres.render_shadow(data);
    }

    fn gui(&mut self, application: &visula::Application, context: &egui::Context) {
        egui::Window::new("Debug").show(context, |ui| {
            self.rendering_controls.gui(application, ui);
        });
    }
}

fn main() {
    visula::run(Simulation::new);
}
