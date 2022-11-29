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
    @builtin(position) projected_position: vec4<f32>,
    @location(0) alpha: f32,
};

struct Line {
    start: vec3<f32>,
    end: vec3<f32>,
    width: f32,
    alpha: f32,
};

fn offset(pos: vec3<f32>, direction: vec3<f32>, unit_offset: vec3<f32>) -> vec3<f32> {
    let view: vec3<f32> = normalize(pos - u_globals.camera_position.xyz);
    let right: vec3<f32> = normalize(cross(direction, view));
    let up: vec3<f32> = normalize(cross(right, view));

    let transform: mat3x3<f32> = mat3x3<f32>(right, up, view);

    return transform * unit_offset;
}

fn linef(
    length_weight: f32,
    width_weight: f32,
    line1: Line,
) -> VertexOutput {
    var output: VertexOutput;
    output.alpha = line1.alpha;

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

    output.projected_position = u_globals.transform * vec4<f32>(vertexPosition, 1.0);

    return output;
}

@vertex
fn vs_main(
    @location(0) length_weight: f32,
    @location(1) width_weight: f32,
) -> VertexOutput {
    var line_input: Line;
    return linef(length_weight, width_weight, line_input);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.8, 0.7, 0.6, input.alpha);
}
