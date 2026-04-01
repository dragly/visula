@group(0) @binding(0)
var ssao_input: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(vertex_index & 1u)) * 4.0 - 1.0;
    let y = f32(i32(vertex_index >> 1u)) * 4.0 - 1.0;
    return vec4<f32>(x, -y, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) f32 {
    let coord = vec2<i32>(frag_coord.xy);
    let tex_size = vec2<i32>(textureDimensions(ssao_input));

    var result = 0.0;
    var count = 0.0;
    for (var x = -2; x <= 2; x++) {
        for (var y = -2; y <= 2; y++) {
            let sample_coord = coord + vec2<i32>(x, y);
            if sample_coord.x >= 0 && sample_coord.x < tex_size.x &&
               sample_coord.y >= 0 && sample_coord.y < tex_size.y {
                result += textureLoad(ssao_input, sample_coord, 0).r;
                count += 1.0;
            }
        }
    }
    return result / count;
}
