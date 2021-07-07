[[block]]
struct Globals {
    view_matrix: mat4x4<f32>;
    transform: mat4x4<f32>;
    camera_center: vec4<f32>;
    camera_view_vector: vec4<f32>;
    camera_position: vec4<f32>;
    camera_up: vec4<f32>;
};

[[group(0), binding(0)]]
var<uniform> u_globals: Globals;

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
};

[[stage(vertex)]]
fn vs_main(
    [[location(0)]] position: vec3<f32>,
    [[location(1)]] position: vec3<f32>,
    [[location(2)]] color: vec4<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = u_globals.transform * vec4<f32>(position, 1.0);
    out.color = color;
    return out;
}

[[stage(fragment)]]
fn fs_main(vertex: VertexOutput) -> [[location(0)]] vec4<f32> {
    return vertex.color;
}
