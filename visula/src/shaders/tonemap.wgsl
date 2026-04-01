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
var hdr_texture: texture_2d<f32>;

@group(0) @binding(1)
var hdr_sampler: sampler;

struct TonemapParams {
    mode: u32,
    ssao_enabled: u32,
    bloom_enabled: u32,
    _pad: f32,
};

@group(0) @binding(2)
var<uniform> params: TonemapParams;

@group(0) @binding(3)
var ssao_texture: texture_2d<f32>;

@group(0) @binding(4)
var bloom_texture: texture_2d<f32>;

fn aces_filmic(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn reinhard(x: vec3<f32>) -> vec3<f32> {
    return x / (x + vec3<f32>(1.0));
}

@fragment
fn fs_main(in: FullscreenOutput) -> @location(0) vec4<f32> {
    var color = textureSample(hdr_texture, hdr_sampler, in.uv).rgb;

    if params.ssao_enabled == 1u {
        let coord = vec2<i32>(in.position.xy);
        let ao = textureLoad(ssao_texture, coord, 0).r;
        color = color * ao;
    }

    if params.bloom_enabled == 1u {
        let bloom = textureSample(bloom_texture, hdr_sampler, in.uv).rgb;
        color = color + bloom;
    }

    if params.mode == 1u {
        color = reinhard(color);
    } else if params.mode == 2u {
        color = aces_filmic(color);
    }

    return vec4<f32>(color, 1.0);
}
