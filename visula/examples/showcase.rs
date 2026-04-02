use bytemuck::{Pod, Zeroable};
use glam::{Quat, Vec3, Vec4};

use visula::{
    primitives::mesh_primitive::MeshVertexAttributes, Expression, InstanceBuffer,
    InstanceDeviceExt, LineGeometry, LineMaterial, Lines, MeshGeometry, MeshMaterial, MeshPipeline,
    RenderData, Renderable, RenderingControls, ShadowRenderData, SphereGeometry, SphereMaterial,
    SpherePrimitive, Spheres,
};
use visula_derive::Instance;
use wgpu::util::DeviceExt;

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct LineData {
    start: [f32; 3],
    end: [f32; 3],
    _padding: [f32; 2],
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum ColorMode {
    FlatColor,
    Normal,
    Position,
    DirectionalLit,
    Lit,
}

impl std::fmt::Display for ColorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ColorMode::FlatColor => write!(f, "Flat Color"),
            ColorMode::Normal => write!(f, "Normal"),
            ColorMode::Position => write!(f, "Position"),
            ColorMode::DirectionalLit => write!(f, "Directional Lit"),
            ColorMode::Lit => write!(f, "Lit"),
        }
    }
}

struct SphereVariants {
    flat: Spheres,
    normal: Spheres,
    position: Spheres,
    directional_lit: Spheres,
    lit: Spheres,
}

struct LineVariants {
    flat: Lines,
    normal: Lines,
    position: Lines,
    directional_lit: Lines,
    lit: Lines,
}

struct Simulation {
    color_mode: ColorMode,
    sphere_variants: SphereVariants,
    _sphere_buffer: InstanceBuffer<SpherePrimitive>,
    line_variants: LineVariants,
    _line_buffer: InstanceBuffer<LineData>,
    mesh: MeshPipeline,
    ground: MeshPipeline,
    rendering_controls: RenderingControls,
}

#[derive(Debug)]
struct Error {}

fn create_sphere_variant(
    app: &visula::Application,
    sphere_buffer: &InstanceBuffer<SpherePrimitive>,
    color_expr: Expression,
) -> Result<Spheres, visula_core::ShaderError> {
    let sphere = sphere_buffer.instance();
    Spheres::new(
        &app.rendering_descriptor(),
        &SphereGeometry {
            position: sphere.position,
            radius: sphere.radius,
            color: sphere.color,
        },
        &SphereMaterial { color: color_expr },
    )
}

impl Simulation {
    fn new(application: &mut visula::Application) -> Result<Simulation, Error> {
        let sphere_buffer: InstanceBuffer<SpherePrimitive> =
            application.device.create_instance_buffer();

        let sphere_variants = SphereVariants {
            flat: create_sphere_variant(application, &sphere_buffer, Expression::InputColor)
                .unwrap(),
            normal: create_sphere_variant(
                application,
                &sphere_buffer,
                Expression::Normal * 0.5 + 0.5,
            )
            .unwrap(),
            position: create_sphere_variant(
                application,
                &sphere_buffer,
                Expression::Position * 0.1 + 0.5,
            )
            .unwrap(),
            directional_lit: create_sphere_variant(
                application,
                &sphere_buffer,
                Expression::InputColor.directional_lit(),
            )
            .unwrap(),
            lit: create_sphere_variant(application, &sphere_buffer, Expression::InputColor.lit())
                .unwrap(),
        };

        let line_buffer: InstanceBuffer<LineData> = application.device.create_instance_buffer();

        let create_line_variant = |app: &visula::Application,
                                   buf: &InstanceBuffer<LineData>,
                                   color_expr: Expression|
         -> Lines {
            let line = buf.instance();
            Lines::new(
                &app.rendering_descriptor(),
                &LineGeometry {
                    start: line.start,
                    end: line.end,
                    width: 0.3.into(),
                    color: Vec3::new(0.8, 0.8, 0.8).into(),
                },
                &LineMaterial { color: color_expr },
            )
            .unwrap()
        };

        let line_variants = LineVariants {
            flat: create_line_variant(application, &line_buffer, Expression::InputColor),
            normal: create_line_variant(application, &line_buffer, Expression::Normal * 0.5 + 0.5),
            position: create_line_variant(
                application,
                &line_buffer,
                Expression::Position * 0.1 + 0.5,
            ),
            directional_lit: create_line_variant(
                application,
                &line_buffer,
                Expression::InputColor.directional_lit(),
            ),
            lit: create_line_variant(application, &line_buffer, Expression::InputColor.lit()),
        };

        let sphere_positions: Vec<[f32; 3]> = vec![
            [0.0, 0.0, 0.0],
            [-3.0, 0.0, 0.0],
            [0.0, 3.0, 0.0],
            [0.0, 0.0, 3.0],
        ];
        let line_data: Vec<LineData> = sphere_positions
            .windows(2)
            .map(|pair| LineData {
                start: pair[0],
                end: pair[1],
                _padding: [0.0; 2],
            })
            .collect();
        line_buffer.update(&application.device, &application.queue, &line_data);

        let mesh = MeshPipeline::new(
            &application.rendering_descriptor(),
            &MeshGeometry {
                position: Vec3::new(4.0, 0.0, 0.0).into(),
                rotation: Quat::IDENTITY.into(),
                scale: Vec3::ONE.into(),
            },
            &MeshMaterial {
                color: Expression::from(Vec4::new(0.8, 0.3, 0.2, 1.0)).lit(),
            },
        )
        .unwrap();

        let mut ground = MeshPipeline::new(
            &application.rendering_descriptor(),
            &MeshGeometry {
                position: Vec3::new(0.0, -2.0, 0.0).into(),
                rotation: Quat::IDENTITY.into(),
                scale: Vec3::ONE.into(),
            },
            &MeshMaterial {
                color: Expression::from(Vec4::new(0.9, 0.9, 0.9, 1.0)).lit(),
            },
        )
        .unwrap();

        let half = 15.0f32;
        let ground_vertices: Vec<MeshVertexAttributes> = vec![
            MeshVertexAttributes {
                position: [-half, 0.0, -half],
                normal: [0.0, 1.0, 0.0],
                uv: [0.0, 0.0],
                color: [255, 255, 255, 255],
            },
            MeshVertexAttributes {
                position: [half, 0.0, -half],
                normal: [0.0, 1.0, 0.0],
                uv: [1.0, 0.0],
                color: [255, 255, 255, 255],
            },
            MeshVertexAttributes {
                position: [half, 0.0, half],
                normal: [0.0, 1.0, 0.0],
                uv: [1.0, 1.0],
                color: [255, 255, 255, 255],
            },
            MeshVertexAttributes {
                position: [-half, 0.0, half],
                normal: [0.0, 1.0, 0.0],
                uv: [0.0, 1.0],
                color: [255, 255, 255, 255],
            },
        ];
        let ground_indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];
        ground.vertex_buffer =
            application
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Ground vertex buffer"),
                    contents: bytemuck::cast_slice(&ground_vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
        ground.index_buffer =
            application
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Ground index buffer"),
                    contents: bytemuck::cast_slice(&ground_indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
        ground.vertex_count = ground_indices.len();

        let sphere_data: Vec<SpherePrimitive> = sphere_positions
            .iter()
            .zip(&[
                [0.8, 0.2, 0.2],
                [0.2, 0.8, 0.2],
                [0.2, 0.2, 0.8],
                [0.8, 0.8, 0.2],
            ])
            .zip(&[1.0f32, 0.8, 0.6, 1.2])
            .map(|((pos, color), radius)| SpherePrimitive {
                position: *pos,
                radius: *radius,
                color: *color,
                padding: 0.0,
            })
            .collect();
        sphere_buffer.update(&application.device, &application.queue, &sphere_data);

        Ok(Simulation {
            color_mode: ColorMode::Lit,
            sphere_variants,
            _sphere_buffer: sphere_buffer,
            line_variants,
            _line_buffer: line_buffer,
            mesh,
            ground,
            rendering_controls: RenderingControls::new(),
        })
    }
}

impl visula::Simulation for Simulation {
    type Error = Error;

    fn update(&mut self, application: &mut visula::Application) {
        self.rendering_controls.update(application);
    }

    fn render(&mut self, data: &mut RenderData) {
        match self.color_mode {
            ColorMode::FlatColor => self.sphere_variants.flat.render(data),
            ColorMode::Normal => self.sphere_variants.normal.render(data),
            ColorMode::Position => self.sphere_variants.position.render(data),
            ColorMode::DirectionalLit => self.sphere_variants.directional_lit.render(data),
            ColorMode::Lit => self.sphere_variants.lit.render(data),
        }
        match self.color_mode {
            ColorMode::FlatColor => self.line_variants.flat.render(data),
            ColorMode::Normal => self.line_variants.normal.render(data),
            ColorMode::Position => self.line_variants.position.render(data),
            ColorMode::DirectionalLit => self.line_variants.directional_lit.render(data),
            ColorMode::Lit => self.line_variants.lit.render(data),
        }
        self.mesh.render(data);
        self.ground.render(data);
    }

    fn render_shadow(&mut self, data: &mut ShadowRenderData) {
        self.sphere_variants.flat.render_shadow(data);
        self.line_variants.flat.render_shadow(data);
    }

    fn gui(&mut self, application: &visula::Application, context: &egui::Context) {
        egui::Window::new("Showcase").show(context, |ui| {
            ui.label("Shading mode");
            for mode in [
                ColorMode::FlatColor,
                ColorMode::Normal,
                ColorMode::Position,
                ColorMode::DirectionalLit,
                ColorMode::Lit,
            ] {
                if ui
                    .selectable_label(self.color_mode == mode, mode.to_string())
                    .clicked()
                {
                    self.color_mode = mode;
                }
            }

            ui.separator();
            self.rendering_controls.gui(application, ui);
        });
    }
}

fn main() {
    visula::run(|app| Simulation::new(app).expect("Initializing simulation failed"));
}
