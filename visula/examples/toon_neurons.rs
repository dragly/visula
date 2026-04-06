use bytemuck::{Pod, Zeroable};
use glam::{Quat, Vec3};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use visula::{
    primitives::{generate_plane, generate_sphere, generate_torus},
    CylinderGeometry, CylinderMaterial, Cylinders, Expression, InstanceBuffer, InstanceDeviceExt,
    MeshGeometry, MeshMaterial, MeshPipeline, RenderData, Renderable, RenderingControls,
    ShadowRenderData, SphereGeometry, SphereMaterial, SpherePrimitive, Spheres,
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
    _padding: f32,
}

struct NeuronConfig {
    position: Vec3,
    scale: f32,
    color: [f32; 3],
}

struct GenerationSettings {
    seed: u64,
    arm_count: i32,
    dendrite_length: f32,
    dendrite_radius: f32,
    dendrite_segments: i32,
    branch_depth: i32,
    wander: f32,
    taper: f32,
    axon_radius: f32,
    soma_brightness: f32,
    needs_regenerate: bool,
}

impl Default for GenerationSettings {
    fn default() -> Self {
        Self {
            seed: 42,
            arm_count: 9,
            dendrite_length: 1.0,
            dendrite_radius: 1.0,
            dendrite_segments: 14,
            branch_depth: 2,
            wander: 0.18,
            taper: 0.8,
            axon_radius: 0.07,
            soma_brightness: 1.4,
            needs_regenerate: false,
        }
    }
}

impl GenerationSettings {
    fn gui(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;
        let mut seed_i32 = self.seed as i32;
        changed |= ui
            .add(egui::Slider::new(&mut seed_i32, 0..=1000).text("Seed"))
            .changed();
        self.seed = seed_i32 as u64;

        changed |= ui
            .add(egui::Slider::new(&mut self.arm_count, 3..=15).text("Arms"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut self.dendrite_length, 0.3..=3.0).text("Dendrite length"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut self.dendrite_radius, 0.3..=3.0).text("Dendrite radius"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut self.dendrite_segments, 4..=30).text("Dendrite segments"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut self.branch_depth, 0..=4).text("Branch depth"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut self.wander, 0.0..=0.5).text("Wander"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut self.taper, 0.0..=1.0).text("Taper"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut self.axon_radius, 0.02..=0.3).text("Axon radius"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut self.soma_brightness, 0.5..=2.0).text("Soma brightness"))
            .changed();

        if changed {
            self.needs_regenerate = true;
        }
    }
}

fn hex_to_rgb(hex: u32) -> [f32; 3] {
    [
        ((hex >> 16) & 0xFF) as f32 / 255.0,
        ((hex >> 8) & 0xFF) as f32 / 255.0,
        (hex & 0xFF) as f32 / 255.0,
    ]
}

fn hex_to_rgba(hex: u32) -> [u8; 4] {
    [
        ((hex >> 16) & 0xFF) as u8,
        ((hex >> 8) & 0xFF) as u8,
        (hex & 0xFF) as u8,
        255,
    ]
}

struct DendriteResult {
    points: Vec<Vec3>,
    radii: Vec<f32>,
}

fn build_dendrite(
    rng: &mut impl Rng,
    origin: Vec3,
    dir: Vec3,
    length: f32,
    base_radius: f32,
    segments: usize,
    settings: &GenerationSettings,
) -> DendriteResult {
    let mut points = vec![origin];
    let step_len = length / segments as f32;
    let current_dir = dir.normalize();

    let up = Vec3::Y;
    let mut perp1 = current_dir.cross(up).normalize();
    if perp1.length() < 0.1 {
        perp1 = Vec3::X;
    }
    let perp2 = current_dir.cross(perp1).normalize();

    let wander_strength = length * settings.wander;
    let mut drift_x = 0.0_f32;
    let mut drift_y = 0.0_f32;

    let mut radii = vec![base_radius];

    for i in 1..=segments {
        let t = i as f32 / segments as f32;
        drift_x += (rng.random::<f32>() - 0.5) * wander_strength;
        drift_y += (rng.random::<f32>() - 0.5) * wander_strength;
        if rng.random::<f32>() < 0.15 {
            drift_x += (rng.random::<f32>() - 0.5) * wander_strength * 2.0;
            drift_y += (rng.random::<f32>() - 0.5) * wander_strength * 2.0;
        }
        let p = origin + current_dir * step_len * i as f32 + perp1 * drift_x + perp2 * drift_y;
        points.push(p);

        let r = base_radius * (1.0 - t * settings.taper) * (0.9 + rng.random::<f32>() * 0.2);
        radii.push(r);
    }

    DendriteResult { points, radii }
}

struct DendriteParams {
    origin: Vec3,
    dir: Vec3,
    length: f32,
    base_radius: f32,
    segments: usize,
    color: [f32; 3],
    depth: usize,
    max_depth: usize,
}

fn collect_dendrite_cylinders(
    rng: &mut impl Rng,
    cylinders: &mut Vec<CylinderData>,
    joints: &mut Vec<SpherePrimitive>,
    params: DendriteParams,
    settings: &GenerationSettings,
) {
    let DendriteParams {
        origin,
        dir,
        length,
        base_radius,
        segments,
        color,
        depth,
        max_depth,
    } = params;
    let segs = if depth == 0 { segments } else { 10 };
    let dend = build_dendrite(rng, origin, dir, length, base_radius, segs, settings);

    for i in 0..dend.points.len() - 1 {
        cylinders.push(CylinderData {
            start: dend.points[i].into(),
            start_radius: dend.radii[i],
            end: dend.points[i + 1].into(),
            end_radius: dend.radii[i + 1],
            color,
            _padding: 0.0,
        });

        if i > 0 {
            joints.push(SpherePrimitive {
                position: dend.points[i].into(),
                radius: dend.radii[i],
                color,
                padding: 0.0,
            });
        }
    }

    if let Some(tip) = dend.points.last() {
        let tip_r = dend.radii.last().copied().unwrap_or(0.02) * 1.4;
        joints.push(SpherePrimitive {
            position: (*tip).into(),
            radius: tip_r,
            color,
            padding: 0.0,
        });
    }

    if depth >= max_depth {
        return;
    }

    let branch_count = if depth == 0 {
        1 + rng.random_range(0..3)
    } else if rng.random::<f32>() > 0.4 {
        1
    } else {
        0
    };

    for _ in 0..branch_count {
        let t = 0.3 + rng.random::<f32>() * 0.5;
        let idx = (t * (dend.points.len() - 1) as f32) as usize;
        let idx = idx.min(dend.points.len() - 1);
        let branch_pt = dend.points[idx];

        let tangent = if idx + 1 < dend.points.len() {
            (dend.points[idx + 1] - dend.points[idx]).normalize()
        } else {
            dir.normalize()
        };

        let angle = (rng.random::<f32>() - 0.5) * 2.5;
        let rot1 = Quat::from_axis_angle(Vec3::Y, angle);
        let rot2 = Quat::from_axis_angle(Vec3::X, (rng.random::<f32>() - 0.5) * 0.8);
        let br_dir = (rot2 * rot1 * tangent).normalize();
        let br_len = length * (0.35 + rng.random::<f32>() * 0.3);
        let br_r = dend.radii[idx] * 0.65;

        collect_dendrite_cylinders(
            rng,
            cylinders,
            joints,
            DendriteParams {
                origin: branch_pt,
                dir: br_dir,
                length: br_len,
                base_radius: br_r,
                segments: 8,
                color,
                depth: depth + 1,
                max_depth,
            },
            settings,
        );
    }
}

struct NeuronGeometry {
    cylinders: Vec<CylinderData>,
    joints: Vec<SpherePrimitive>,
}

fn generate_neuron_geometry(
    rng: &mut impl Rng,
    neuron: &NeuronConfig,
    settings: &GenerationSettings,
) -> NeuronGeometry {
    let mut cylinders = Vec::new();
    let mut joints = Vec::new();
    let origin = neuron.position + Vec3::Y * neuron.scale * 0.8;
    let arm_count = settings.arm_count + rng.random_range(0..4);

    for i in 0..arm_count {
        let th = (i as f32 / arm_count as f32) * std::f32::consts::TAU
            + (rng.random::<f32>() - 0.5) * 0.5;
        let ph = 0.4 + rng.random::<f32>() * 1.1;
        let dir = Vec3::new(ph.sin() * th.cos(), ph.cos() * 0.2, ph.sin() * th.sin()).normalize();
        let len = neuron.scale * (1.6 + rng.random::<f32>() * 2.0) * settings.dendrite_length;
        let base_r = (neuron.scale * 0.13 + rng.random::<f32>() * neuron.scale * 0.07)
            * settings.dendrite_radius;

        let dendrite_start = origin + dir * neuron.scale * 0.7;
        collect_dendrite_cylinders(
            rng,
            &mut cylinders,
            &mut joints,
            DendriteParams {
                origin: dendrite_start,
                dir,
                length: len,
                base_radius: base_r,
                segments: settings.dendrite_segments as usize,
                color: neuron.color,
                depth: 0,
                max_depth: settings.branch_depth as usize,
            },
            settings,
        );
    }

    NeuronGeometry { cylinders, joints }
}

fn generate_axon_cylinders(
    start: Vec3,
    end: Vec3,
    color: [f32; 3],
    radius: f32,
) -> Vec<CylinderData> {
    let a = start + Vec3::Y * 0.7;
    let b = end + Vec3::Y * 0.7;
    let mid = (a + b) * 0.5 + Vec3::Y * 0.5;

    let segments = 20;
    let mut cylinders = Vec::new();

    for i in 0..segments {
        let t0 = i as f32 / segments as f32;
        let t1 = (i + 1) as f32 / segments as f32;
        let p0 = quadratic_bezier(a, mid, b, t0);
        let p1 = quadratic_bezier(a, mid, b, t1);

        cylinders.push(CylinderData {
            start: p0.into(),
            start_radius: radius,
            end: p1.into(),
            end_radius: radius,
            color,
            _padding: 0.0,
        });
    }

    cylinders
}

fn quadratic_bezier(a: Vec3, b: Vec3, c: Vec3, t: f32) -> Vec3 {
    let u = 1.0 - t;
    a * u * u + b * 2.0 * u * t + c * t * t
}

fn green_neurons() -> [NeuronConfig; 4] {
    [
        NeuronConfig {
            position: Vec3::new(-5.0, 0.0, -3.0),
            scale: 1.0,
            color: hex_to_rgb(0x7cb83c),
        },
        NeuronConfig {
            position: Vec3::new(-3.0, 0.0, 2.5),
            scale: 1.1,
            color: hex_to_rgb(0x8acc4a),
        },
        NeuronConfig {
            position: Vec3::new(-7.0, 0.0, 1.5),
            scale: 0.95,
            color: hex_to_rgb(0x6aaa34),
        },
        NeuronConfig {
            position: Vec3::new(-4.0, 0.0, -6.5),
            scale: 1.05,
            color: hex_to_rgb(0x90d84e),
        },
    ]
}

fn blue_neurons() -> [NeuronConfig; 3] {
    [
        NeuronConfig {
            position: Vec3::new(4.0, 0.0, 3.0),
            scale: 0.9,
            color: hex_to_rgb(0x3898cc),
        },
        NeuronConfig {
            position: Vec3::new(6.0, 0.0, -2.0),
            scale: 0.95,
            color: hex_to_rgb(0x2a80b8),
        },
        NeuronConfig {
            position: Vec3::new(3.0, 0.0, -5.0),
            scale: 1.0,
            color: hex_to_rgb(0x48a8dd),
        },
    ]
}

struct GeneratedBufferData {
    dendrites: Vec<CylinderData>,
    joints: Vec<SpherePrimitive>,
    axons: Vec<CylinderData>,
    somas: Vec<SpherePrimitive>,
}

fn generate_all(settings: &GenerationSettings) -> GeneratedBufferData {
    let mut rng = StdRng::seed_from_u64(settings.seed);
    let greens = green_neurons();
    let blues = blue_neurons();
    let all_neurons: Vec<&NeuronConfig> = greens.iter().chain(blues.iter()).collect();

    let mut dendrites = Vec::new();
    let mut joints = Vec::new();
    for neuron in &all_neurons {
        let geo = generate_neuron_geometry(&mut rng, neuron, settings);
        dendrites.extend(geo.cylinders);
        joints.extend(geo.joints);
    }

    let gp: Vec<Vec3> = greens.iter().map(|n| n.position).collect();
    let bp: Vec<Vec3> = blues.iter().map(|n| n.position).collect();

    let mut axons = Vec::new();
    let axon_connections = [
        (gp[0], gp[1], hex_to_rgb(0x80cc38)),
        (gp[1], gp[2], hex_to_rgb(0x70bb28)),
        (gp[0], gp[3], hex_to_rgb(0x90dd48)),
        (gp[2], gp[3], hex_to_rgb(0x80cc38)),
        (bp[0], bp[1], hex_to_rgb(0x3898cc)),
        (bp[1], bp[2], hex_to_rgb(0x2a88bb)),
        (bp[0], bp[2], hex_to_rgb(0x48aadd)),
        (gp[1], bp[0], hex_to_rgb(0xee9933)),
    ];
    for (start, end, color) in &axon_connections {
        axons.extend(generate_axon_cylinders(
            *start,
            *end,
            *color,
            settings.axon_radius,
        ));
    }

    let somas: Vec<SpherePrimitive> = all_neurons
        .iter()
        .map(|neuron| {
            let soma_pos = neuron.position + Vec3::Y * neuron.scale * 0.8;
            let b = settings.soma_brightness;
            let color = [
                (neuron.color[0] * b).min(1.0),
                (neuron.color[1] * b).min(1.0),
                (neuron.color[2] * b).min(1.0),
            ];
            SpherePrimitive {
                position: soma_pos.into(),
                radius: neuron.scale,
                color,
                padding: 0.0,
            }
        })
        .collect();

    GeneratedBufferData {
        dendrites,
        joints,
        axons,
        somas,
    }
}

struct ToonNeurons {
    dendrite_cylinders: Cylinders,
    dendrite_buffer: InstanceBuffer<CylinderData>,
    joint_spheres: Spheres,
    joint_buffer: InstanceBuffer<SpherePrimitive>,
    axon_cylinders: Cylinders,
    axon_buffer: InstanceBuffer<CylinderData>,
    soma_spheres: Spheres,
    soma_buffer: InstanceBuffer<SpherePrimitive>,
    dish_floor: MeshPipeline,
    dish_rim: MeshPipeline,
    synapse_meshes: Vec<MeshPipeline>,
    rendering_controls: RenderingControls,
    generation_settings: GenerationSettings,
}

impl ToonNeurons {
    fn new(app: &mut visula::Application) -> Self {
        let device = &app.device;
        let settings = GenerationSettings::default();
        let data = generate_all(&settings);

        let dendrite_buffer: InstanceBuffer<CylinderData> = device.create_instance_buffer();
        dendrite_buffer.update(device, &app.queue, &data.dendrites);

        let dendrite_instance = dendrite_buffer.instance();
        let dendrite_cylinders = Cylinders::new(
            &app.rendering_descriptor(),
            &CylinderGeometry {
                start: dendrite_instance.start,
                end: dendrite_instance.end,
                start_radius: dendrite_instance.start_radius,
                end_radius: dendrite_instance.end_radius,
                color: dendrite_instance.color,
            },
            &CylinderMaterial {
                color: Expression::InputColor.toon_lit(),
            },
        )
        .unwrap();

        let joint_buffer: InstanceBuffer<SpherePrimitive> = device.create_instance_buffer();
        joint_buffer.update(device, &app.queue, &data.joints);

        let joint_instance = joint_buffer.instance();
        let joint_spheres = Spheres::new(
            &app.rendering_descriptor(),
            &SphereGeometry {
                position: joint_instance.position,
                radius: joint_instance.radius,
                color: joint_instance.color,
            },
            &SphereMaterial {
                color: Expression::InputColor.toon_lit(),
            },
        )
        .unwrap();

        let axon_buffer: InstanceBuffer<CylinderData> = device.create_instance_buffer();
        axon_buffer.update(device, &app.queue, &data.axons);

        let axon_instance = axon_buffer.instance();
        let axon_cylinders = Cylinders::new(
            &app.rendering_descriptor(),
            &CylinderGeometry {
                start: axon_instance.start,
                end: axon_instance.end,
                start_radius: axon_instance.start_radius,
                end_radius: axon_instance.end_radius,
                color: axon_instance.color,
            },
            &CylinderMaterial {
                color: Expression::InputColor.toon_lit(),
            },
        )
        .unwrap();

        let soma_buffer: InstanceBuffer<SpherePrimitive> = device.create_instance_buffer();
        soma_buffer.update(device, &app.queue, &data.somas);

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

        let greens = green_neurons();
        let blues = blue_neurons();
        let gp: Vec<Vec3> = greens.iter().map(|n| n.position).collect();
        let bp: Vec<Vec3> = blues.iter().map(|n| n.position).collect();

        let synapse_positions = [
            ((gp[0] + gp[1]) * 0.5 + Vec3::Y * 0.9, hex_to_rgba(0xccff44)),
            ((gp[1] + gp[2]) * 0.5 + Vec3::Y * 0.9, hex_to_rgba(0xbbee33)),
            ((bp[0] + bp[1]) * 0.5 + Vec3::Y * 0.9, hex_to_rgba(0x66ccff)),
            ((gp[1] + bp[0]) * 0.5 + Vec3::Y * 0.9, hex_to_rgba(0xffbb44)),
        ];

        let mut synapse_meshes = Vec::new();
        for (pos, color) in &synapse_positions {
            let (vertices, indices) = generate_sphere(0.18, 12, 12, *color);
            let mut mesh = MeshPipeline::new(
                &app.rendering_descriptor(),
                &MeshGeometry {
                    position: (*pos).into(),
                    rotation: Quat::IDENTITY.into(),
                    scale: Vec3::ONE.into(),
                },
                &MeshMaterial {
                    color: Expression::InputColor.toon_lit(),
                },
            )
            .unwrap();
            mesh.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Synapse vertex buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            mesh.index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Synapse index buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            mesh.vertex_count = indices.len();
            synapse_meshes.push(mesh);
        }

        let floor_color = hex_to_rgba(0x1a3318);
        let (floor_verts, floor_indices) = generate_plane(30.0, 30.0, floor_color);

        let mut dish_floor = MeshPipeline::new(
            &app.rendering_descriptor(),
            &MeshGeometry {
                position: Vec3::new(0.0, -0.3, 0.0).into(),
                rotation: Quat::IDENTITY.into(),
                scale: Vec3::ONE.into(),
            },
            &MeshMaterial {
                color: Expression::InputColor.toon_lit(),
            },
        )
        .unwrap();

        dish_floor.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Dish floor vertex buffer"),
            contents: bytemuck::cast_slice(&floor_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        dish_floor.index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Dish floor index buffer"),
            contents: bytemuck::cast_slice(&floor_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        dish_floor.vertex_count = floor_indices.len();

        let rim_color = hex_to_rgba(0x5a7a50);
        let (rim_verts, rim_indices) = generate_torus(14.0, 0.4, 64, 16, rim_color);

        let mut dish_rim = MeshPipeline::new(
            &app.rendering_descriptor(),
            &MeshGeometry {
                position: Vec3::new(0.0, -0.1, 0.0).into(),
                rotation: Quat::IDENTITY.into(),
                scale: Vec3::ONE.into(),
            },
            &MeshMaterial {
                color: Expression::InputColor.toon_lit(),
            },
        )
        .unwrap();
        dish_rim.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Dish rim vertex buffer"),
            contents: bytemuck::cast_slice(&rim_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        dish_rim.index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Dish rim index buffer"),
            contents: bytemuck::cast_slice(&rim_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        dish_rim.vertex_count = rim_indices.len();

        app.post_processor.config.bloom = Some(visula::post_process::config::BloomConfig {
            threshold: 0.8,
            intensity: 0.5,
            mip_levels: 5,
        });
        app.post_processor
            .enable_bloom(&app.device, app.config.width, app.config.height);

        app.post_processor.config.outline = visula::post_process::config::OutlineConfig {
            enabled: true,
            color: [0.0, 0.0, 0.0],
            thickness: 6.0,
            depth_threshold: 0.1,
        };

        ToonNeurons {
            dendrite_cylinders,
            dendrite_buffer,
            joint_spheres,
            joint_buffer,
            axon_cylinders,
            axon_buffer,
            soma_spheres,
            soma_buffer,
            dish_floor,
            dish_rim,
            synapse_meshes,
            rendering_controls: RenderingControls::new(),
            generation_settings: settings,
        }
    }

    fn regenerate(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let data = generate_all(&self.generation_settings);
        self.dendrite_buffer.update(device, queue, &data.dendrites);
        self.joint_buffer.update(device, queue, &data.joints);
        self.axon_buffer.update(device, queue, &data.axons);
        self.soma_buffer.update(device, queue, &data.somas);
    }
}

impl visula::Simulation for ToonNeurons {
    type Error = ();

    fn update(&mut self, application: &mut visula::Application) {
        self.rendering_controls.update(application);
        if self.generation_settings.needs_regenerate {
            self.generation_settings.needs_regenerate = false;
            self.regenerate(&application.device, &application.queue);
        }
    }

    fn render(&mut self, data: &mut RenderData) {
        self.dish_floor.render(data);
        self.dish_rim.render(data);
        self.dendrite_cylinders.render(data);
        self.joint_spheres.render(data);
        self.axon_cylinders.render(data);
        self.soma_spheres.render(data);
        for mesh in &self.synapse_meshes {
            mesh.render(data);
        }
    }

    fn render_shadow(&mut self, data: &mut ShadowRenderData) {
        self.dendrite_cylinders.render_shadow(data);
        self.joint_spheres.render_shadow(data);
        self.axon_cylinders.render_shadow(data);
        self.soma_spheres.render_shadow(data);
    }

    fn gui(&mut self, application: &visula::Application, context: &egui::Context) {
        egui::Window::new("Rendering").show(context, |ui| {
            self.rendering_controls.gui(application, ui);
        });
        egui::Window::new("Generation").show(context, |ui| {
            self.generation_settings.gui(ui);
        });
    }

    fn clear_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: 0.055,
            g: 0.118,
            b: 0.055,
            a: 1.0,
        }
    }
}

fn main() {
    visula::run(ToonNeurons::new);
}
