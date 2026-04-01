@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(vertex_index & 1u)) * 4.0 - 1.0;
    let y = f32(i32(vertex_index >> 1u)) * 4.0 - 1.0;
    return vec4<f32>(x, -y, 0.0, 1.0);
}

@group(0) @binding(0)
var depth_msaa: texture_depth_multisampled_2d;

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @builtin(frag_depth) f32 {
    let coord = vec2<i32>(frag_coord.xy);
    var min_depth = 1.0;
    for (var i = 0; i < 4; i++) {
        let sample = textureLoad(depth_msaa, coord, i);
        min_depth = min(min_depth, sample);
    }
    return min_depth;
}
