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
    [[builtin(position)]] projected_position: vec4<f32>;
    [[location(0)]] alpha: f32;
};

struct Line {
    start: vec3<f32>;
    end: vec3<f32>;
    width: f32;
    alpha: f32;
};

fn offset(pos: vec3<f32>, direction: vec3<f32>, unit_offset: vec3<f32>) -> vec3<f32> {
    let view: vec3<f32> = normalize(pos - u_globals.camera_position.xyz);
    let right: vec3<f32> = normalize(cross(direction, view));
    let up: vec3<f32> = normalize(cross(right, view));

    let transform: mat3x3<f32> = mat3x3<f32>(right, up, view);

    return transform * unit_offset;
}

fn line(
    length_weight: f32,
    width_weight: f32,
    line: Line,
) -> VertexOutput {
    var output: VertexOutput;
    output.alpha = line.alpha;

    let width_half = line.width / 2.0;
    let left = vec3<f32>(-width_half, 0.0, 0.0);
    let right = vec3<f32>(width_half, 0.0, 0.0);
    let direction = line.end - line.start;
    let offset_left_start = offset(line.start, direction, left);
    let offset_right_start = offset(line.start, direction, right);
    let offset_left_end = offset(line.end, direction, left);
    let offset_right_end = offset(line.end, direction, right);

    let offset_left = (1.0 - length_weight) * offset_left_start + length_weight * offset_left_end;
    let offset_right = (1.0 - length_weight) * offset_right_start + length_weight * offset_right_end;

    let pos = (
        (1.0 - length_weight) * line.start +
        length_weight * line.end +
        (1.0 - width_weight) * offset_left +
        (width_weight) * offset_right
    );

    let vertexPosition = pos;

    output.projected_position = u_globals.transform * vec4<f32>(vertexPosition, 1.0);

    return output;
}

[[stage(vertex)]]
fn vs_main(
    [[location(0)]] length_weight: f32,
    [[location(1)]] width_weight: f32,
) -> VertexOutput {
    var line_input: Line;
    return line(length_weight, width_weight, line_input);
}

[[stage(fragment)]]
fn fs_main(input: VertexOutput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(0.8, 0.7, 0.6, input.alpha);
}
