struct Globals {
    view_matrix: mat4x4<f32>,
    transform: mat4x4<f32>,
    camera_center: vec4<f32>,
    camera_view_vector: vec4<f32>,
    camera_position: vec4<f32>,
    camera_up: vec4<f32>,
};

@group(0)
@binding(0)
var<uniform> u_globals: Globals;

struct VertexOutput {
    @builtin(position) proj_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

struct Polygon {
    color: vec4<f32>,
    position: vec3<f32>,
};

fn polygon_vertex(vertex_position: vec2<f32>, polygon: Polygon) -> VertexOutput {
    var output: VertexOutput;
    let world_pos = polygon.position + vec3<f32>(vertex_position.x, vertex_position.y, 0.0);
    output.proj_position = u_globals.transform * vec4<f32>(world_pos, 1.0);
    output.color = polygon.color;
    return output;
}

@vertex
fn vs_main(
    @location(0) vertex_position: vec2<f32>,
) -> VertexOutput {
    var polygon: Polygon;
    return polygon_vertex(vertex_position, polygon);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if in.color.a < 0.01 {
        discard;
    }
    return in.color;
}
