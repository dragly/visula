struct FullscreenOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_fullscreen(@builtin(vertex_index) vertex_index: u32) -> FullscreenOutput {
    let x = f32(i32(vertex_index & 1u)) * 4.0 - 1.0;
    let y = f32(i32(vertex_index >> 1u)) * 4.0 - 1.0;
    var output: FullscreenOutput;
    output.position = vec4<f32>(x, -y, 0.0, 1.0);
    output.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (-y * 0.5 + 0.5));
    return output;
}

@group(0) @binding(0)
var normal_texture: texture_2d<f32>;

@group(0) @binding(1)
var depth_ms_texture: texture_depth_multisampled_2d;

struct OutlineParams {
    color: vec3<f32>,
    thickness: f32,
    depth_threshold: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(2)
var<uniform> params: OutlineParams;

fn detect_outline(coord: vec2<i32>) -> f32 {
    let n_c = textureLoad(normal_texture, coord, 0).xyz;
    if dot(n_c, n_c) < 0.01 {
        return 0.0;
    }

    let s = i32(params.thickness);

    let n_r = textureLoad(normal_texture, coord + vec2<i32>(s, 0), 0).xyz;
    let n_l = textureLoad(normal_texture, coord + vec2<i32>(-s, 0), 0).xyz;
    let n_b = textureLoad(normal_texture, coord + vec2<i32>(0, s), 0).xyz;
    let n_t = textureLoad(normal_texture, coord + vec2<i32>(0, -s), 0).xyz;
    let bg_edge = f32(dot(n_r, n_r) < 0.01) + f32(dot(n_l, n_l) < 0.01)
                + f32(dot(n_b, n_b) < 0.01) + f32(dot(n_t, n_t) < 0.01);

    let d_r = textureLoad(depth_ms_texture, coord + vec2<i32>(s, 0), 0);
    let d_l = textureLoad(depth_ms_texture, coord + vec2<i32>(-s, 0), 0);
    let d_b = textureLoad(depth_ms_texture, coord + vec2<i32>(0, s), 0);
    let d_t = textureLoad(depth_ms_texture, coord + vec2<i32>(0, -s), 0);
    let dh = d_r - d_l;
    let dv = d_b - d_t;
    let depth_edge = (dh * dh + dv * dv) * 1e6;

    return max(
        step(0.5, bg_edge),
        smoothstep(params.depth_threshold * 0.5, params.depth_threshold, depth_edge),
    );
}

@fragment
fn fs_main(in: FullscreenOutput) -> @location(0) vec4<f32> {
    let coord = vec2<i32>(in.position.xy);
    let edge = detect_outline(coord);
    return vec4<f32>(params.color, edge);
}
