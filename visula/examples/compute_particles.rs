use bytemuck::{Pod, Zeroable};
use visula::{
    glam::Vec3, wgpu, wgpu::util::DeviceExt, InstanceBuffer, PrepareData, RenderData, Renderable,
    ShadowRenderData, SphereGeometry, SphereMaterial, Spheres,
};
use visula_derive::Instance;

const COUNT: u32 = 20_000;
const WORKGROUP_SIZE: u32 = 64;

const SHADER: &str = "
struct Particle {
    x: f32, y: f32, z: f32,
    r: f32, g: f32, b: f32,
    radius: f32,
}

@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= arrayLength(&particles) {
        return;
    }
    var particle = particles[id.x];
    let angle = 0.04 / (1.0 + 0.2 * length(vec2<f32>(particle.x, particle.z)));
    let x = cos(angle) * particle.x - sin(angle) * particle.z;
    particle.z = sin(angle) * particle.x + cos(angle) * particle.z;
    particle.x = x;
    particles[id.x] = particle;
}
";

#[repr(C)]
#[derive(Clone, Copy, Debug, Instance, Pod, Zeroable)]
struct Particle {
    position: Vec3,
    color: Vec3,
    radius: f32,
}

struct Simulation {
    spheres: Spheres,
    particles: InstanceBuffer<Particle>,
    state: wgpu::Buffer,
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
}

impl Simulation {
    fn new(application: &mut visula::Application) -> Simulation {
        let device = &application.device;
        let golden_angle = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
        let initial: Vec<Particle> = (0..COUNT)
            .map(|i| {
                let t = i as f32 / COUNT as f32;
                let distance = 20.0 * t.sqrt();
                let angle = i as f32 * golden_angle;
                Particle {
                    position: Vec3::new(
                        distance * angle.cos(),
                        0.3 * (i as f32 * 0.37).sin(),
                        distance * angle.sin(),
                    ),
                    color: Vec3::new(1.0, 0.75, 0.4).lerp(Vec3::new(0.3, 0.5, 1.0), t),
                    radius: 0.15,
                }
            })
            .collect();

        let particles = InstanceBuffer::new_with_init(device, &initial);
        let state = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("particle state"),
            contents: bytemuck::cast_slice(&initial),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particles"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("particles"),
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("particles"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: state.as_entire_binding(),
            }],
        });

        let instance = particles.instance();
        let spheres = Spheres::new(
            &application.rendering_descriptor(),
            &SphereGeometry {
                position: instance.position,
                radius: instance.radius,
                color: instance.color,
            },
            &SphereMaterial::default(),
        )
        .unwrap();

        Simulation {
            spheres,
            particles,
            state,
            pipeline,
            bind_group,
        }
    }
}

impl visula::Simulation for Simulation {
    fn prepare(&mut self, data: &mut PrepareData) {
        {
            let mut pass = data
                .encoder
                .begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("particles"),
                    timestamp_writes: None,
                });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(COUNT.div_ceil(WORKGROUP_SIZE), 1, 1);
        }
        data.encoder.copy_buffer_to_buffer(
            &self.state,
            0,
            &self.particles.inner.borrow().buffer,
            0,
            None,
        );
    }

    fn render(&mut self, data: &mut RenderData) {
        self.spheres.render(data);
    }

    fn render_shadow(&mut self, data: &mut ShadowRenderData) {
        self.spheres.render_shadow(data);
    }
}

fn main() {
    visula::run(Simulation::new);
}
