struct Particle {
  pos : vec2<f32>,
  vel : vec2<f32>,
};

@group(0) @binding(0) var<storage, read> particlesSrc : array<Particle>;
@group(0) @binding(1) var<storage, read_write> particlesDst : array<Particle>;

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    var compute: Particle;
    // modification happens here
}
