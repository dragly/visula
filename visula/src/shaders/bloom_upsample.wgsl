@group(0) @binding(0)
var src_texture: texture_2d<f32>;

@group(0) @binding(1)
var src_sampler: sampler;

struct Params {
    texel_size: vec2<f32>,
    intensity: f32,
    _pad: f32,
};

@group(0) @binding(2)
var<uniform> params: Params;

struct FullscreenOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> FullscreenOutput {
    let x = f32(i32(vertex_index & 1u)) * 4.0 - 1.0;
    let y = f32(i32(vertex_index >> 1u)) * 4.0 - 1.0;
    var output: FullscreenOutput;
    output.position = vec4<f32>(x, -y, 0.0, 1.0);
    output.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (-y * 0.5 + 0.5));
    return output;
}

@fragment
fn fs_main(in: FullscreenOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let t = params.texel_size;

    // 9-tap tent filter upsample
    var color = textureSample(src_texture, src_sampler, uv + vec2<f32>(-t.x, -t.y));
    color += textureSample(src_texture, src_sampler, uv + vec2<f32>(0.0, -t.y)) * 2.0;
    color += textureSample(src_texture, src_sampler, uv + vec2<f32>(t.x, -t.y));
    color += textureSample(src_texture, src_sampler, uv + vec2<f32>(-t.x, 0.0)) * 2.0;
    color += textureSample(src_texture, src_sampler, uv) * 4.0;
    color += textureSample(src_texture, src_sampler, uv + vec2<f32>(t.x, 0.0)) * 2.0;
    color += textureSample(src_texture, src_sampler, uv + vec2<f32>(-t.x, t.y));
    color += textureSample(src_texture, src_sampler, uv + vec2<f32>(0.0, t.y)) * 2.0;
    color += textureSample(src_texture, src_sampler, uv + vec2<f32>(t.x, t.y));

    color /= 16.0;
    color *= params.intensity;

    return color;
}
