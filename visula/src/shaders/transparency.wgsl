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
var accumulation_texture: texture_2d<f32>;

@group(0) @binding(1)
var weight_texture: texture_2d<f32>;

@fragment
fn fs_main(input: FullscreenOutput) -> @location(0) vec4<f32> {
    let coord = vec2<i32>(input.position.xy);
    let accumulation = textureLoad(accumulation_texture, coord, 0);
    let weight = textureLoad(weight_texture, coord, 0).r;
    return vec4<f32>(accumulation.rgb / max(weight, 1e-5), 1.0 - accumulation.a);
}
