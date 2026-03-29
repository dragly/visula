use bytemuck::{Pod, Zeroable};
use visula::{
    application::Application, BlitPipeline, ComputePass, CustomEvent, RenderData, StorageBuffer,
    StorageTexture,
};
use wgpu::util::DeviceExt;
use winit::event::{ElementState, Event, MouseButton, WindowEvent};

const MAX_NEURONS: usize = 128;
const MAX_SEGMENTS: usize = 2048;
const MAX_CONNECTIONS: usize = 64;

const CAM: [f32; 3] = [0.0, 3.2, 3.8];
const TAR: [f32; 3] = [0.0, 0.0, -0.2];
const FOV_VAL: f32 = 1.2;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct GpuNeuron {
    pos: [f32; 4],
    info: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct GpuSegment {
    base: [f32; 4],
    tip: [f32; 4],
    info: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct BvhNode {
    aabb_min: [f32; 4],
    aabb_max: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct GpuConnection {
    data: [f32; 4],
    info: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct SdfParams {
    resolution: [f32; 2],
    time: f32,
    neuron_count: f32,
    cam_pos: [f32; 4],
    cam_target: [f32; 4],
    cam_fov: f32,
    bvh_count: f32,
    conn_count: f32,
    _pad: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Excitatory,
    Inhibitory,
    Connect,
}

struct Dendrite {
    angle: f32,
    target_angle: f32,
    length: f32,
    radius: f32,
}

struct NeuronState {
    x: f32,
    z: f32,
    neuron_type: f32,
    id: u32,
    dendrites: Vec<Dendrite>,
    alive: f32,
    target_alive: f32,
    max_r: f32,
}

struct ConnectionState {
    from_id: u32,
    to_id: u32,
    progress: f32,
    target_progress: f32,
}

#[derive(Debug)]
struct Error {}

struct Simulation {
    neurons: Vec<NeuronState>,
    connections: Vec<ConnectionState>,
    next_id: u32,
    mode: Mode,
    time: f32,
    last_mouse_pos: Option<(f64, f64)>,
    drag_from: Option<u32>,
    dragging: bool,

    neurons_buf: StorageBuffer<GpuNeuron>,
    segments_buf: StorageBuffer<GpuSegment>,
    bvh_buf: StorageBuffer<BvhNode>,
    connections_buf: StorageBuffer<GpuConnection>,
    params_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    params_bind_group_layout: wgpu::BindGroupLayout,
    storage_bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    output_texture: StorageTexture,
    compute: ComputePass,
    blit: BlitPipeline,
    cached_handles: [uuid::Uuid; 4],
    #[allow(dead_code)]
    surface_format: wgpu::TextureFormat,
}

fn build_bvh(neurons: &[NeuronState]) -> (Vec<BvhNode>, usize) {
    let alive: Vec<usize> = neurons
        .iter()
        .enumerate()
        .filter(|(_, n)| n.alive > 0.01)
        .map(|(i, _)| i)
        .collect();
    if alive.is_empty() {
        return (vec![BvhNode::zeroed(); 1], 0);
    }

    struct TempNode {
        mn_x: f32,
        mn_y: f32,
        mn_z: f32,
        left: i32,
        mx_x: f32,
        mx_y: f32,
        mx_z: f32,
        right: i32,
    }

    let mut nodes: Vec<TempNode> = Vec::new();

    fn leaf(nodes: &mut Vec<TempNode>, neurons: &[NeuronState], ni: usize) -> usize {
        let n = &neurons[ni];
        let r = 0.12 * n.alive + n.max_r;
        let idx = nodes.len();
        nodes.push(TempNode {
            mn_x: n.x - r,
            mn_y: -0.01,
            mn_z: n.z - r,
            left: -1,
            mx_x: n.x + r,
            mx_y: 0.3 + r,
            mx_z: n.z + r,
            right: ni as i32,
        });
        idx
    }

    fn build(
        nodes: &mut Vec<TempNode>,
        neurons: &[NeuronState],
        ids: &mut [usize],
    ) -> usize {
        if ids.len() == 1 {
            return leaf(nodes, neurons, ids[0]);
        }
        if ids.is_empty() {
            return 0;
        }
        let mut c_mn_x = f32::MAX;
        let mut c_mx_x = f32::MIN;
        let mut c_mn_z = f32::MAX;
        let mut c_mx_z = f32::MIN;
        for &i in ids.iter() {
            let n = &neurons[i];
            c_mn_x = c_mn_x.min(n.x);
            c_mx_x = c_mx_x.max(n.x);
            c_mn_z = c_mn_z.min(n.z);
            c_mx_z = c_mx_z.max(n.z);
        }
        let sort_x = (c_mx_x - c_mn_x) >= (c_mx_z - c_mn_z);
        if sort_x {
            ids.sort_by(|&a, &b| neurons[a].x.partial_cmp(&neurons[b].x).unwrap());
        } else {
            ids.sort_by(|&a, &b| neurons[a].z.partial_cmp(&neurons[b].z).unwrap());
        }
        let mid = ids.len() / 2;
        let (left_ids, right_ids) = ids.split_at_mut(mid);
        let l = build(nodes, neurons, left_ids);
        let r = build(nodes, neurons, right_ids);
        let idx = nodes.len();
        nodes.push(TempNode {
            mn_x: nodes[l].mn_x.min(nodes[r].mn_x),
            mn_y: nodes[l].mn_y.min(nodes[r].mn_y),
            mn_z: nodes[l].mn_z.min(nodes[r].mn_z),
            left: l as i32,
            mx_x: nodes[l].mx_x.max(nodes[r].mx_x),
            mx_y: nodes[l].mx_y.max(nodes[r].mx_y),
            mx_z: nodes[l].mx_z.max(nodes[r].mx_z),
            right: r as i32,
        });
        idx
    }

    let mut ids = alive;
    let root = build(&mut nodes, neurons, &mut ids);

    let mut ordered: Vec<usize> = Vec::new();
    let mut remap = vec![0usize; nodes.len()];
    fn visit(
        nodes: &[TempNode],
        i: usize,
        ordered: &mut Vec<usize>,
        remap: &mut [usize],
    ) {
        let ni = ordered.len();
        remap[i] = ni;
        ordered.push(i);
        let nd = &nodes[i];
        if nd.left >= 0 {
            visit(nodes, nd.left as usize, ordered, remap);
            visit(nodes, nd.right as usize, ordered, remap);
        }
    }
    visit(&nodes, root, &mut ordered, &mut remap);

    let count = ordered.len();
    let mut result = Vec::with_capacity(count.max(1));
    for &oi in &ordered {
        let nd = &nodes[oi];
        let (left, right) = if nd.left >= 0 {
            (remap[nd.left as usize] as f32, remap[nd.right as usize] as f32)
        } else {
            (-1.0f32, nd.right as f32)
        };
        result.push(BvhNode {
            aabb_min: [nd.mn_x, nd.mn_y, nd.mn_z, f32::from_bits(left.to_bits())],
            aabb_max: [nd.mx_x, nd.mx_y, nd.mx_z, f32::from_bits(right.to_bits())],
        });
    }
    if result.is_empty() {
        result.push(BvhNode::zeroed());
    }
    (result, count)
}

impl Simulation {
    fn new(application: &mut Application) -> Result<Self, Error> {
        let device = &application.device;
        let w = application.config.width.max(1);
        let h = application.config.height.max(1);

        let neurons_buf = StorageBuffer::<GpuNeuron>::new(device, "neurons");
        let segments_buf = StorageBuffer::<GpuSegment>::new(device, "segments");
        let bvh_buf = StorageBuffer::<BvhNode>::new(device, "bvh");
        let connections_buf = StorageBuffer::<GpuConnection>::new(device, "connections");

        let params_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("params layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let storage_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("storage layout"),
                entries: &[
                    storage_entry(0),
                    storage_entry(1),
                    storage_entry(2),
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    storage_entry(5),
                ],
            });

        let params_data = SdfParams {
            resolution: [w as f32, h as f32],
            time: 0.0,
            neuron_count: 0.0,
            cam_pos: [CAM[0], CAM[1], CAM[2], 0.0],
            cam_target: [TAR[0], TAR[1], TAR[2], 0.0],
            cam_fov: FOV_VAL,
            bvh_count: 0.0,
            conn_count: 0.0,
            _pad: 0.0,
        };
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("params"),
            contents: bytemuck::cast_slice(&[params_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let output_texture = StorageTexture::new(device, w, h);

        let surface_format = application.config.view_formats[0];

        let compute = ComputePass::new(
            device,
            include_str!("sdf_neurons_compute.wgsl"),
            "main",
            &[&storage_bind_group_layout],
        );

        let blit = BlitPipeline::new(device, surface_format, &output_texture.sample_bind_group_layout);

        let bind_group = create_bind_group(
            device,
            &storage_bind_group_layout,
            &neurons_buf,
            &segments_buf,
            &bvh_buf,
            &params_buffer,
            &output_texture,
            &connections_buf,
        );

        let cached_handles = [
            neurons_buf.handle(),
            segments_buf.handle(),
            bvh_buf.handle(),
            connections_buf.handle(),
        ];

        let mut sim = Self {
            neurons: Vec::new(),
            connections: Vec::new(),
            next_id: 0,
            mode: Mode::Excitatory,
            time: 0.0,
            last_mouse_pos: None,
            drag_from: None,
            dragging: false,
            neurons_buf,
            segments_buf,
            bvh_buf,
            connections_buf,
            params_buffer,
            params_bind_group_layout,
            storage_bind_group_layout,
            bind_group,
            output_texture,
            compute,
            blit,
            cached_handles,
            surface_format,
        };
        sim.do_demo();
        Ok(sim)
    }

    fn make_neuron(&mut self, wx: f32, wz: f32, neuron_type: f32) {
        if self.neurons.len() >= MAX_NEURONS {
            return;
        }
        let id = self.next_id;
        self.next_id += 1;
        let nd = 4 + (rand_f32(id as f32) * 2.0) as usize;
        let mut dendrites = Vec::new();
        for i in 0..nd {
            let a = (i as f32 / nd as f32) * std::f32::consts::TAU
                + (rand_f32(id as f32 + i as f32 * 7.3) - 0.5) * 0.4;
            dendrites.push(Dendrite {
                angle: a,
                target_angle: a,
                length: 0.25 + rand_f32(id as f32 * 3.1 + i as f32) * 0.35,
                radius: 0.016 + rand_f32(id as f32 * 5.7 + i as f32 * 2.3) * 0.01,
            });
        }
        self.neurons.push(NeuronState {
            x: wx,
            z: wz,
            neuron_type,
            id,
            dendrites,
            alive: 0.0,
            target_alive: 1.0,
            max_r: 0.5,
        });
    }

    fn find_at(&self, wx: f32, wz: f32) -> Option<u32> {
        let mut best = None;
        let mut best_dist = 0.3f32;
        for n in &self.neurons {
            let d = ((n.x - wx).powi(2) + (n.z - wz).powi(2)).sqrt();
            if d < best_dist {
                best_dist = d;
                best = Some(n.id);
            }
        }
        best
    }

    fn find_by_id(&self, id: u32) -> Option<usize> {
        self.neurons.iter().position(|n| n.id == id)
    }

    fn add_connection(&mut self, from_id: u32, to_id: u32) {
        if from_id == to_id || self.connections.len() >= MAX_CONNECTIONS {
            return;
        }
        if self
            .connections
            .iter()
            .any(|c| c.from_id == from_id && c.to_id == to_id)
        {
            return;
        }
        let from_idx = match self.find_by_id(from_id) {
            Some(i) => i,
            None => return,
        };
        let to_idx = match self.find_by_id(to_id) {
            Some(i) => i,
            None => return,
        };
        self.connections.push(ConnectionState {
            from_id,
            to_id,
            progress: 0.0,
            target_progress: 1.0,
        });
        let ia = (self.neurons[from_idx].z - self.neurons[to_idx].z)
            .atan2(self.neurons[from_idx].x - self.neurons[to_idx].x);
        for d in &mut self.neurons[to_idx].dendrites {
            let mut df = d.target_angle - ia;
            while df > std::f32::consts::PI {
                df -= std::f32::consts::TAU;
            }
            while df < -std::f32::consts::PI {
                df += std::f32::consts::TAU;
            }
            if df.abs() < 0.6 {
                d.target_angle += if df > 0.0 { 0.5 } else { -0.5 };
            }
        }
    }

    fn do_demo(&mut self) {
        self.neurons.clear();
        self.connections.clear();
        self.next_id = 0;
        for i in 0..24 {
            let ring = i / 8;
            let angle =
                (i % 8) as f32 / 8.0 * std::f32::consts::TAU + ring as f32 * 0.3;
            let radius = 0.5 + ring as f32 * 0.7;
            let t = if rand_f32(i as f32 * 13.7) > 0.65 {
                1.0
            } else {
                0.0
            };
            self.make_neuron(
                angle.cos() * radius + (rand_f32(i as f32 * 2.1) - 0.5) * 0.3,
                angle.sin() * radius + (rand_f32(i as f32 * 3.3) - 0.5) * 0.3,
                t,
            );
        }
        let n = self.neurons.len();
        let mut pairs = Vec::new();
        for i in 0..n {
            for j in (i + 1)..n {
                let dx = self.neurons[i].x - self.neurons[j].x;
                let dz = self.neurons[i].z - self.neurons[j].z;
                if (dx * dx + dz * dz).sqrt() < 0.8
                    && rand_f32(i as f32 * 17.0 + j as f32 * 31.0) > 0.4
                {
                    pairs.push((self.neurons[i].id, self.neurons[j].id));
                }
            }
        }
        for (f, t) in pairs {
            self.add_connection(f, t);
        }
    }

    fn screen_to_world(&self, px: f64, py: f64, width: f32, height: f32) -> (f32, f32) {
        let uvx = (px as f32 - 0.5 * width) / height;
        let uvy = (0.5 * height - py as f32) / height;

        let wx = TAR[0] - CAM[0];
        let wy = TAR[1] - CAM[1];
        let wz = TAR[2] - CAM[2];
        let wl = (wx * wx + wy * wy + wz * wz).sqrt();
        let (wx, wy, wz) = (wx / wl, wy / wl, wz / wl);

        let (ux, uz) = (-wz, wx);
        let ul = (ux * ux + uz * uz).sqrt();
        let (ux, uz) = (ux / ul, uz / ul);
        let uy = 0.0f32;

        let vx = uy * wz - uz * wy;
        let vy = uz * wx - ux * wz;
        let vz = ux * wy - uy * wx;

        let rl = (uvx * uvx + uvy * uvy + FOV_VAL * FOV_VAL).sqrt();
        let (lx, ly, lz) = (uvx / rl, uvy / rl, FOV_VAL / rl);
        let rdx = ux * lx + vx * ly + wx * lz;
        let rdy = uy * lx + vy * ly + wy * lz;
        let rdz = uz * lx + vz * ly + wz * lz;

        if rdy.abs() < 0.0001 {
            return (0.0, 0.0);
        }
        let t = -CAM[1] / rdy;
        if t < 0.0 {
            return (0.0, 0.0);
        }
        (CAM[0] + rdx * t, CAM[2] + rdz * t)
    }

    fn rebuild_bind_group(&mut self, device: &wgpu::Device) {
        let current = [
            self.neurons_buf.handle(),
            self.segments_buf.handle(),
            self.bvh_buf.handle(),
            self.connections_buf.handle(),
        ];
        if current != self.cached_handles {
            self.bind_group = create_bind_group(
                device,
                &self.storage_bind_group_layout,
                &self.neurons_buf,
                &self.segments_buf,
                &self.bvh_buf,
                &self.params_buffer,
                &self.output_texture,
                &self.connections_buf,
            );
            self.cached_handles = current;
        }
    }
}

fn storage_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn create_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    neurons: &StorageBuffer<GpuNeuron>,
    segments: &StorageBuffer<GpuSegment>,
    bvh: &StorageBuffer<BvhNode>,
    params: &wgpu::Buffer,
    texture: &StorageTexture,
    connections: &StorageBuffer<GpuConnection>,
) -> wgpu::BindGroup {
    let tex_view = texture
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("SDF bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: neurons.buffer().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: segments.buffer().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: bvh.buffer().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(&tex_view),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: connections.buffer().as_entire_binding(),
            },
        ],
    })
}

fn rand_f32(seed: f32) -> f32 {
    ((seed * 127.1 + 311.7).sin() * 43758.5453).fract().abs()
}

impl visula::Simulation for Simulation {
    type Error = Error;

    fn handle_event(&mut self, application: &mut Application, event: &Event<CustomEvent>) {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                self.last_mouse_pos = Some((position.x, position.y));
            }
            Event::WindowEvent {
                event:
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: MouseButton::Left,
                        ..
                    },
                ..
            } => {
                if let Some((mx, my)) = self.last_mouse_pos {
                    let w = application.config.width as f32;
                    let h = application.config.height as f32;
                    let (wx, wz) = self.screen_to_world(mx, my, w, h);
                    match self.mode {
                        Mode::Connect => {
                            if let Some(id) = self.find_at(wx, wz) {
                                self.drag_from = Some(id);
                                self.dragging = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
            Event::WindowEvent {
                event:
                    WindowEvent::MouseInput {
                        state: ElementState::Released,
                        button: MouseButton::Left,
                        ..
                    },
                ..
            } => {
                if let Some((mx, my)) = self.last_mouse_pos {
                    let w = application.config.width as f32;
                    let h = application.config.height as f32;
                    let (wx, wz) = self.screen_to_world(mx, my, w, h);
                    if self.mode == Mode::Connect && self.dragging {
                        if let Some(from_id) = self.drag_from {
                            if let Some(to_id) = self.find_at(wx, wz) {
                                if to_id != from_id {
                                    self.add_connection(from_id, to_id);
                                }
                            }
                        }
                        self.dragging = false;
                        self.drag_from = None;
                    } else if self.mode != Mode::Connect {
                        let t = if self.mode == Mode::Inhibitory {
                            1.0
                        } else {
                            0.0
                        };
                        self.make_neuron(wx, wz, t);
                    }
                }
            }
            _ => {}
        }
    }

    fn update(&mut self, application: &mut Application) {
        let dt = 0.016f32;
        self.time += dt;

        for n in &mut self.neurons {
            n.alive += (n.target_alive - n.alive) * 3.0 * dt;
            for d in &mut n.dendrites {
                d.angle += (d.target_angle - d.angle) * 2.5 * dt;
            }
        }
        for c in &mut self.connections {
            c.progress += (c.target_progress - c.progress) * 1.5 * dt;
        }

        let mut n_data = vec![GpuNeuron::zeroed(); self.neurons.len().max(1)];
        let mut s_data: Vec<GpuSegment> = Vec::new();

        for (i, nn) in self.neurons.iter_mut().enumerate() {
            let al = nn.alive;
            let seg_off = s_data.len();
            let s_r = 0.12 * al;

            for d in &nn.dendrites {
                let ag = d.angle;
                let ln = d.length * al;
                let rd = d.radius * al;
                let dx = ag.cos();
                let dy = 0.12f32;
                let dz = ag.sin();
                let dl = (dx * dx + dy * dy + dz * dz).sqrt();
                let (dx, dy, dz) = (dx / dl, dy / dl, dz / dl);

                if s_data.len() < MAX_SEGMENTS {
                    s_data.push(GpuSegment {
                        base: [
                            nn.x + dx * s_r * 0.35,
                            0.1 + dy * s_r * 0.35,
                            nn.z + dz * s_r * 0.35,
                            rd * 1.2,
                        ],
                        tip: [
                            nn.x + dx * (s_r * 0.35 + ln),
                            (0.1 + dy * (s_r * 0.35 + ln)).max(0.008),
                            nn.z + dz * (s_r * 0.35 + ln),
                            rd * 0.2,
                        ],
                        info: [0.05 * al, 0.0, 0.0, 0.0],
                    });
                }
            }

            let seg_count = s_data.len() - seg_off;
            let mut max_r = 0.0f32;
            for j in seg_off..s_data.len() {
                let s = &s_data[j];
                let d1 = s.base[0] - nn.x;
                let d2 = s.base[2] - nn.z;
                let d3 = s.tip[0] - nn.x;
                let d4 = s.tip[2] - nn.z;
                max_r = max_r
                    .max((d1 * d1 + d2 * d2).sqrt())
                    .max((d3 * d3 + d4 * d4).sqrt());
            }
            nn.max_r = max_r + 0.05;

            if i < n_data.len() {
                n_data[i] = GpuNeuron {
                    pos: [nn.x, 0.1, nn.z, nn.neuron_type],
                    info: [al, 0.12, seg_off as f32, seg_count as f32],
                };
            }
        }

        if s_data.is_empty() {
            s_data.push(GpuSegment::zeroed());
        }

        let (bvh_data, bvh_count) = build_bvh(&self.neurons);

        let mut c_data = vec![GpuConnection::zeroed(); self.connections.len().max(1)];
        let cc = self.connections.len().min(MAX_CONNECTIONS);
        for i in 0..cc {
            let conn = &self.connections[i];
            let from_idx = self.find_by_id(conn.from_id);
            let to_idx = self.find_by_id(conn.to_id);
            if let (Some(fi), Some(ti)) = (from_idx, to_idx) {
                let f = &self.neurons[fi];
                let t = &self.neurons[ti];
                c_data[i] = GpuConnection {
                    data: [f.x, f.z, t.x, t.z],
                    info: [f.neuron_type, conn.progress, 0.0, 0.0],
                };
            }
        }

        let w = application.config.width.max(1);
        let h = application.config.height.max(1);

        let params = SdfParams {
            resolution: [w as f32, h as f32],
            time: self.time,
            neuron_count: self.neurons.len() as f32,
            cam_pos: [CAM[0], CAM[1], CAM[2], 0.0],
            cam_target: [TAR[0], TAR[1], TAR[2], 0.0],
            cam_fov: FOV_VAL,
            bvh_count: bvh_count as f32,
            conn_count: cc as f32,
            _pad: 0.0,
        };

        application
            .queue
            .write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));
        self.neurons_buf
            .update(&application.device, &application.queue, &n_data);
        self.segments_buf
            .update(&application.device, &application.queue, &s_data);
        self.bvh_buf
            .update(&application.device, &application.queue, &bvh_data);
        self.connections_buf
            .update(&application.device, &application.queue, &c_data);

        self.output_texture.resize(&application.device, w, h);
        self.rebuild_bind_group(&application.device);
    }

    fn render(&mut self, data: &mut RenderData) {
        let w = self.output_texture.width;
        let h = self.output_texture.height;
        self.compute.dispatch(
            data.encoder,
            &[&self.bind_group],
            [(w + 7) / 8, (h + 7) / 8, 1],
        );
        self.blit
            .render(data.encoder, data.view, &self.output_texture.sample_bind_group);
    }

    fn gui(&mut self, _application: &Application, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(self.mode == Mode::Excitatory, "● Excitatory")
                    .clicked()
                {
                    self.mode = Mode::Excitatory;
                }
                if ui
                    .selectable_label(self.mode == Mode::Inhibitory, "● Inhibitory")
                    .clicked()
                {
                    self.mode = Mode::Inhibitory;
                }
                ui.separator();
                if ui
                    .selectable_label(self.mode == Mode::Connect, "⟶ Connect")
                    .clicked()
                {
                    self.mode = Mode::Connect;
                }
                ui.separator();
                if ui.button("Clear").clicked() {
                    self.neurons.clear();
                    self.connections.clear();
                    self.next_id = 0;
                }
                if ui.button("Demo").clicked() {
                    self.do_demo();
                }
                ui.separator();
                ui.label(format!(
                    "{} neurons · {} connections",
                    self.neurons.len(),
                    self.connections.len()
                ));
            });
        });
    }

    fn clear_color(&self) -> wgpu::Color {
        wgpu::Color::BLACK
    }
}

fn main() {
    visula::run(|app| Simulation::new(app).expect("Failed to initialize SDF neurons"));
}
