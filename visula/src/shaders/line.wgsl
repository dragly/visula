struct Globals {
    view_matrix: mat4x4<f32>,
    transform: mat4x4<f32>,
    camera_center: vec4<f32>,
    camera_view_vector: vec4<f32>,
    camera_position: vec4<f32>,
    camera_up: vec4<f32>,
    inverse_view_proj: mat4x4<f32>,
    screen_size: vec4<f32>,
    projection_matrix: mat4x4<f32>,
    inverse_projection_matrix: mat4x4<f32>,
};

@group(0)
@binding(0)
var<uniform> u_globals: Globals;

struct VertexOutput {
    @builtin(position) projected_position: vec4<f32>,
    @location(0) instance_color: vec3<f32>,
    @location(1) vertex_position: vec3<f32>,
    @location(2) vertex_normal: vec3<f32>,
};

struct LineGeometry {
    start: vec3<f32>,
    end: vec3<f32>,
    width: f32,
    color: vec3<f32>,
};

struct LineMaterial {
    color: vec3<f32>,
};

fn offset(pos: vec3<f32>, direction: vec3<f32>, unit_offset: vec3<f32>) -> vec3<f32> {
    let view: vec3<f32> = normalize(pos - u_globals.camera_position.xyz);
    let right: vec3<f32> = normalize(cross(direction, view));
    let up: vec3<f32> = normalize(cross(right, view));

    let transform: mat3x3<f32> = mat3x3<f32>(right, up, view);

    return transform * unit_offset;
}

fn linef(
    texture_coordinate: vec2<f32>,
    line1: LineGeometry,
) -> VertexOutput {
    let length_weight = texture_coordinate.x;
    let width_weight = texture_coordinate.y;

    var output: VertexOutput;

    let width_half = line1.width / 2.0;
    let left = vec3<f32>(-width_half, 0.0, 0.0);
    let right = vec3<f32>(width_half, 0.0, 0.0);
    let direction = line1.end - line1.start;
    let offset_left_start = offset(line1.start, direction, left);
    let offset_right_start = offset(line1.start, direction, right);
    let offset_left_end = offset(line1.end, direction, left);
    let offset_right_end = offset(line1.end, direction, right);

    let offset_left = (1.0 - length_weight) * offset_left_start + length_weight * offset_left_end;
    let offset_right = (1.0 - length_weight) * offset_right_start + length_weight * offset_right_end;

    let pos = (
        (1.0 - length_weight) * line1.start +
        length_weight * line1.end +
        (1.0 - width_weight) * offset_left +
        (width_weight) * offset_right
    );

    let vertexPosition = pos;

    let view_dir = normalize(vertexPosition - u_globals.camera_position.xyz);
    let line_right = normalize(cross(direction, view_dir));
    let line_normal = normalize(cross(direction, line_right));

    output.projected_position = u_globals.transform * vec4<f32>(vertexPosition, 1.0);
    output.instance_color = line1.color;
    output.vertex_position = vertexPosition;
    output.vertex_normal = line_normal;

    return output;
}

@vertex
fn vs_main(
    @location(0) texture_coordinate: vec2<f32>,
) -> VertexOutput {
    var line_geometry: LineGeometry;
    return linef(texture_coordinate, line_geometry);
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) normal: vec4<f32>,
};

@fragment
fn fs_main(input: VertexOutput) -> FragmentOutput {
    var _visula_instance_color: vec3<f32> = input.instance_color;
    var _visula_normal: vec3<f32> = normalize(input.vertex_normal);
    var _visula_position: vec3<f32> = input.vertex_position;
    var _visula_view_direction: vec3<f32> = normalize(u_globals.camera_position.xyz - input.vertex_position);
    var line_material: LineMaterial;

    var output: FragmentOutput;
    output.color = vec4<f32>(line_material.color, 1.0);
    output.normal = vec4<f32>(_visula_normal, 0.0);
    return output;
}
