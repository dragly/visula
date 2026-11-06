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

const PI: f32 = 3.14159265358979;
const STRAIGHT_TURN: f32 = 1e-4;
const EDGE_PIXELS: f32 = 2.0;
const JOINT_PIXELS: f32 = 0.05;

struct VertexOutput {
    @builtin(position) projected_position: vec4<f32>,
    @location(0) input_color: vec3<f32>,
    @location(1) vertex_position: vec3<f32>,
    @location(2) vertex_normal: vec3<f32>,
    @location(3) local_position: vec2<f32>,
    @location(4) shape: vec4<f32>,
    @location(5) half_width: f32,
};

struct ArcGeometry {
    start: vec3<f32>,
    end: vec3<f32>,
    turn: f32,
    width: f32,
    normal: vec3<f32>,
    color: vec3<f32>,
};

struct ArcMaterial {
    color: vec3<f32>,
};

fn pixel_size(position: vec3<f32>) -> f32 {
    let clip = u_globals.transform * vec4<f32>(position, 1.0);
    return 2.0 * abs(clip.w) / (u_globals.projection_matrix[1][1] * u_globals.screen_size.y);
}

fn arcf(corner: vec2<f32>, arc: ArcGeometry) -> VertexOutput {
    let chord = arc.end - arc.start;
    let half_length = 0.5 * length(chord);
    let along = chord / max(2.0 * half_length, 1e-12);
    let side = select(1.0, -1.0, arc.turn < 0.0);
    let across = side * normalize(cross(arc.normal, along));
    let middle = 0.5 * (arc.start + arc.end);
    let turn = abs(arc.turn);
    let half_width = 0.5 * arc.width;

    var radius = 0.0;
    var center = 0.0;
    var reach = half_length;
    var low = -half_width;
    var high = half_width;
    if turn > STRAIGHT_TURN {
        radius = half_length / sin(0.5 * turn);
        center = radius * cos(0.5 * turn);
        reach = select(half_length * (1.0 + half_width / radius), radius + half_width, turn > PI);
        low = center - radius - half_width;
        high = half_width * abs(cos(0.5 * turn));
    }

    let base = vec2<f32>(corner.x * reach, select(low, high, corner.y > 0.0));
    let margin = EDGE_PIXELS * pixel_size(middle + along * base.x + across * base.y);
    let local_position = base + corner * margin;
    let position = middle + along * local_position.x + across * local_position.y;
    let facing = select(1.0, -1.0, dot(arc.normal, u_globals.camera_position.xyz - position) < 0.0);

    var output: VertexOutput;
    output.projected_position = u_globals.transform * vec4<f32>(position, 1.0);
    output.input_color = arc.color;
    output.vertex_position = position;
    output.vertex_normal = facing * arc.normal;
    output.local_position = local_position;
    output.shape = vec4<f32>(half_length, radius, center, turn);
    output.half_width = half_width;
    return output;
}

@vertex
fn vs_main(
    @location(0) corner: vec2<f32>,
) -> VertexOutput {
    var arc_geometry: ArcGeometry;
    return arcf(corner, arc_geometry);
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) normal: vec4<f32>,
};

@fragment
fn fs_main(input: VertexOutput) -> FragmentOutput {
    let half_length = input.shape.x;
    let radius = input.shape.y;
    let turn = input.shape.w;
    let local_position = input.local_position;
    let from_center = local_position - vec2<f32>(0.0, input.shape.z);
    let curved = radius > 0.0;

    let offset = select(local_position.y, radius - length(from_center), curved);
    let pixel = max(length(vec2<f32>(dpdx(offset), dpdy(offset))), 1e-12);
    let joint = JOINT_PIXELS * max(length(dpdx(local_position)), length(dpdy(local_position)));

    let start = vec2<f32>(-half_length, -input.shape.z);
    let swept = atan2(start.x * from_center.y - start.y * from_center.x, dot(start, from_center));
    let slack = joint / max(radius, 1e-12);
    let around = select(swept, swept + 2.0 * PI, swept < -slack);
    let inside = select(abs(local_position.x) <= half_length + joint, around <= turn + slack, curved);

    let coverage = clamp(0.5 - (abs(offset) - input.half_width) / pixel, 0.0, 1.0);
    if !inside || coverage <= 0.0 {
        discard;
    }

    var _visula_input_color: vec3<f32> = input.input_color;
    var _visula_normal: vec3<f32> = normalize(input.vertex_normal);
    var _visula_position: vec3<f32> = input.vertex_position;
    var _visula_view_direction: vec3<f32> = normalize(u_globals.camera_position.xyz - input.vertex_position);
    var arc_material: ArcMaterial;

    var output: FragmentOutput;
    output.color = vec4<f32>(arc_material.color, coverage);
    output.normal = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    return output;
}
