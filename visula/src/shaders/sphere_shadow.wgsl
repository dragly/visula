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
    @builtin(position) proj_position: vec4<f32>,
    @location(0) plane_coord: vec2<f32>,
    @location(1) radius: f32,
    @location(2) vertex_position: vec3<f32>,
    @location(3) instance_position: vec3<f32>,
    @location(4) input_color: vec3<f32>,
};

struct SphereGeometry {
    position: vec3<f32>,
    radius: f32,
    color: vec3<f32>,
};

struct FragmentOutput {
    @builtin(frag_depth) depth: f32,
};

fn spheres_shadow(
    vertex_offset_pre_transform: vec4<f32>,
    sphere: SphereGeometry,
) -> VertexOutput {
    var output: VertexOutput;

    let light_dir = normalize(u_light.direction);
    let up_candidate = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(dot(light_dir, vec3<f32>(0.0, 1.0, 0.0))) > 0.99);
    let right = normalize(cross(light_dir, up_candidate));
    let up = normalize(cross(right, light_dir));

    let transform = mat3x3<f32>(right, up, light_dir);
    let vertex_offset = sphere.radius * (transform * vertex_offset_pre_transform.xyz);
    let vertex_position = vertex_offset + sphere.position;

    output.proj_position = u_light.light_view_proj * vec4<f32>(vertex_position, 1.0);
    output.plane_coord = vertex_offset_pre_transform.xy;
    output.radius = sphere.radius;
    output.vertex_position = vertex_position;
    output.instance_position = sphere.position;
    output.input_color = sphere.color;

    return output;
}

@vertex
fn vs_main(
    @location(0) vertex_offset_pre_transform: vec4<f32>,
) -> VertexOutput {
    var sphere_geometry: SphereGeometry;
    return spheres_shadow(vertex_offset_pre_transform, sphere_geometry);
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let ray_direction = normalize(u_light.direction);
    let ray_origin = in.vertex_position - in.instance_position;

    let radius = in.radius;
    let E = ray_origin;
    let D = ray_direction;

    let r2 = radius * radius;
    let a = dot(D, D);
    let b = 2.0 * dot(E, D);
    let c = dot(E, E) - r2;
    let d = b * b - 4.0 * a * c;

    if d < 0.0 {
        discard;
    }

    let sqrtd = sqrt(d);
    let t1 = (-b - sqrtd) / (2.0 * a);
    let t2 = (-b + sqrtd) / (2.0 * a);
    let t = min(t1, t2);

    let intersection = in.instance_position + ray_origin + t * ray_direction;
    let clip = u_light.light_view_proj * vec4<f32>(intersection, 1.0);

    var output: FragmentOutput;
    output.depth = clip.z / clip.w;
    return output;
}
