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
    [[builtin(position)]] projected_position: vec4<f32>;
};

struct Line {
    start: vec3<f32>;
    end: vec3<f32>;
    width: f32;
};

fn line(
    length_weight: f32,
    width_weight: f32,
    line: Line,
) -> VertexOutput {
    var output: VertexOutput;

    let width_half = line.width / 2.0;

    let view_start: vec3<f32> = normalize(line.start - u_globals.camera_position.xyz);
    let view_end: vec3<f32> = normalize(line.end - u_globals.camera_position.xyz);
    let right_start: vec3<f32> = normalize(cross(view_start, u_globals.camera_up.xyz));
    let right_end: vec3<f32> = normalize(cross(view_end, u_globals.camera_up.xyz));

    let offset_left_start = -right_start * width_half;
    let offset_right_start = right_start * width_half;
    let offset_left_end = -right_end * width_half;
    let offset_right_end = right_end * width_half;

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
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
}
