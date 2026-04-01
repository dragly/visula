struct Light {
    direction: vec3<f32>,
    _pad0: f32,
    color: vec3<f32>,
    intensity: f32,
    light_view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> u_light: Light;

struct VertexOutput {
    @builtin(position) projected_position: vec4<f32>,
};

struct LineGeometry {
    start: vec3<f32>,
    end: vec3<f32>,
    width: f32,
    color: vec3<f32>,
};

fn offset(pos: vec3<f32>, direction: vec3<f32>, unit_offset: vec3<f32>) -> vec3<f32> {
    let view = normalize(u_light.direction);
    let right = normalize(cross(direction, view));
    let up = normalize(cross(right, view));
    let transform = mat3x3<f32>(right, up, view);
    return transform * unit_offset;
}

@vertex
fn vs_main(
    @location(0) texture_coordinate: vec2<f32>,
) -> VertexOutput {
    var line_geometry: LineGeometry;

    let length_weight = texture_coordinate.x;
    let width_weight = texture_coordinate.y;

    let width_half = line_geometry.width / 2.0;
    let left = vec3<f32>(-width_half, 0.0, 0.0);
    let right = vec3<f32>(width_half, 0.0, 0.0);
    let direction = line_geometry.end - line_geometry.start;
    let offset_left_start = offset(line_geometry.start, direction, left);
    let offset_right_start = offset(line_geometry.start, direction, right);
    let offset_left_end = offset(line_geometry.end, direction, left);
    let offset_right_end = offset(line_geometry.end, direction, right);

    let offset_left = (1.0 - length_weight) * offset_left_start + length_weight * offset_left_end;
    let offset_right = (1.0 - length_weight) * offset_right_start + length_weight * offset_right_end;

    let pos = (
        (1.0 - length_weight) * line_geometry.start +
        length_weight * line_geometry.end +
        (1.0 - width_weight) * offset_left +
        (width_weight) * offset_right
    );

    var output: VertexOutput;
    output.projected_position = u_light.light_view_proj * vec4<f32>(pos, 1.0);
    return output;
}
