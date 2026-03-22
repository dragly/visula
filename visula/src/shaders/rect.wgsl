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
    @location(0) local_coord: vec2<f32>,
    @location(1) half_size: vec2<f32>,
    @location(2) corner_radius: f32,
    @location(3) fill_color: vec4<f32>,
    @location(4) stroke_color: vec4<f32>,
    @location(5) stroke_width: f32,
};

struct Rect {
    position: vec3<f32>,
    size: vec2<f32>,
    corner_radius: f32,
    fill_color: vec4<f32>,
    stroke_color: vec4<f32>,
    stroke_width: f32,
};

fn rounded_rect_sdf(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half_size + vec2<f32>(radius, radius);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

fn rect_vertex(quad_coord: vec2<f32>, rect: Rect) -> VertexOutput {
    var output: VertexOutput;
    let half_size = rect.size * 0.5;
    let extent = half_size + vec2<f32>(rect.stroke_width, rect.stroke_width);
    let world_pos = rect.position + vec3<f32>(
        quad_coord.x * extent.x,
        quad_coord.y * extent.y,
        0.0
    );
    output.proj_position = u_globals.transform * vec4<f32>(world_pos, 1.0);
    output.local_coord = quad_coord * extent;
    output.half_size = half_size;
    output.corner_radius = rect.corner_radius;
    output.fill_color = rect.fill_color;
    output.stroke_color = rect.stroke_color;
    output.stroke_width = rect.stroke_width;
    return output;
}

@vertex
fn vs_main(
    @location(0) quad_coord: vec2<f32>,
) -> VertexOutput {
    var rect: Rect;
    return rect_vertex(quad_coord, rect);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sdf = rounded_rect_sdf(in.local_coord, in.half_size, in.corner_radius);
    let aa = fwidth(sdf);

    var color = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    if in.fill_color.a > 0.0 {
        let fill_alpha = 1.0 - smoothstep(-aa, aa, sdf);
        color = vec4<f32>(in.fill_color.rgb, in.fill_color.a * fill_alpha);
    }

    if in.stroke_color.a > 0.0 && in.stroke_width > 0.0 {
        let stroke_sdf = abs(sdf) - in.stroke_width * 0.5;
        let stroke_alpha = 1.0 - smoothstep(-aa, aa, stroke_sdf);
        let stroke = vec4<f32>(in.stroke_color.rgb, in.stroke_color.a * stroke_alpha);
        color = mix(color, stroke, stroke_alpha);
    }

    if color.a < 0.01 {
        discard;
    }
    return color;
}
