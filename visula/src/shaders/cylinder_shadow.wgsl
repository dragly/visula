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
    @location(0) vertex_position: vec3<f32>,
    @location(1) cyl_start: vec3<f32>,
    @location(2) cyl_end: vec3<f32>,
    @location(3) start_radius: f32,
    @location(4) end_radius: f32,
    @location(5) input_color: vec3<f32>,
};

struct CylinderGeometry {
    start: vec3<f32>,
    end: vec3<f32>,
    start_radius: f32,
    end_radius: f32,
    color: vec3<f32>,
};

struct FragmentOutput {
    @builtin(frag_depth) depth: f32,
};

fn cylinders_shadow(
    vertex_position: vec3<f32>,
    cyl: CylinderGeometry,
) -> VertexOutput {
    var output: VertexOutput;

    let axis = cyl.end - cyl.start;
    let height = length(axis);
    let dir = axis / max(height, 0.0001);

    let up_candidate = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(dot(dir, vec3<f32>(0.0, 1.0, 0.0))) > 0.99);
    let right = normalize(cross(dir, up_candidate));
    let up = normalize(cross(right, dir));

    let max_radius = max(cyl.start_radius, cyl.end_radius);
    let padding = max_radius * 0.5;

    let t = vertex_position.y * 0.5 + 0.5;
    let center = cyl.start + axis * t;
    let world_pos = center
        + right * vertex_position.x * (max_radius + padding)
        + up * vertex_position.z * (max_radius + padding)
        + dir * vertex_position.y * padding;

    output.proj_position = u_light.light_view_proj * vec4<f32>(world_pos, 1.0);
    output.vertex_position = world_pos;
    output.cyl_start = cyl.start;
    output.cyl_end = cyl.end;
    output.start_radius = cyl.start_radius;
    output.end_radius = cyl.end_radius;
    output.input_color = cyl.color;

    return output;
}

@vertex
fn vs_main(
    @location(0) vertex_position: vec3<f32>,
) -> VertexOutput {
    var cylinder_geometry: CylinderGeometry;
    return cylinders_shadow(vertex_position, cylinder_geometry);
}

fn intersect_cone_t(
    ray_origin: vec3<f32>,
    ray_dir: vec3<f32>,
    cyl_start: vec3<f32>,
    cyl_end: vec3<f32>,
    start_radius: f32,
    end_radius: f32,
) -> f32 {
    let axis = cyl_end - cyl_start;
    let height = length(axis);
    let dir = axis / max(height, 0.0001);

    let oc = ray_origin - cyl_start;
    let oc_dot_dir = dot(oc, dir);
    let ray_dot_dir = dot(ray_dir, dir);

    let dr = end_radius - start_radius;
    let slope = dr / max(height, 0.0001);

    let oc_perp = oc - oc_dot_dir * dir;
    let ray_perp = ray_dir - ray_dot_dir * dir;

    let r_func_offset = start_radius + slope * oc_dot_dir;
    let r_func_slope = slope * ray_dot_dir;

    let a = dot(ray_perp, ray_perp) - r_func_slope * r_func_slope;
    let b = 2.0 * (dot(oc_perp, ray_perp) - r_func_offset * r_func_slope);
    let c = dot(oc_perp, oc_perp) - r_func_offset * r_func_offset;

    let discriminant = b * b - 4.0 * a * c;

    var best_t = 1e30;

    if discriminant >= 0.0 {
        let sqrtd = sqrt(discriminant);
        let t1 = (-b - sqrtd) / (2.0 * a);
        let t2 = (-b + sqrtd) / (2.0 * a);

        for (var i = 0; i < 2; i++) {
            let t = select(t2, t1, i == 0);
            let p = oc + t * ray_dir;
            let h = dot(p, dir);
            if h >= 0.0 && h <= height && t > 0.0 && t < best_t {
                best_t = t;
            }
        }
    }

    for (var cap = 0; cap < 2; cap++) {
        let cap_h = select(0.0, height, cap == 1);
        let cap_r = select(start_radius, end_radius, cap == 1);
        let cap_normal_dir = select(-dir, dir, cap == 1);
        let denom = dot(ray_dir, cap_normal_dir);
        if abs(denom) < 0.0001 { continue; }
        let cap_center = cyl_start + cap_h * dir;
        let t = dot(cap_center - ray_origin, cap_normal_dir) / denom;
        let p = ray_origin + t * ray_dir - cap_center;
        if dot(p, p) <= cap_r * cap_r && t > 0.0 && t < best_t {
            best_t = t;
        }
    }

    if best_t > 1e29 { return -1e30; }
    return best_t;
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let ray_direction = normalize(u_light.direction);
    let t = intersect_cone_t(in.vertex_position, ray_direction, in.cyl_start, in.cyl_end, in.start_radius, in.end_radius);

    if t < 0.0 || t > 1e29 {
        discard;
    }

    let hit_pos = in.vertex_position + t * ray_direction;
    let clip = u_light.light_view_proj * vec4<f32>(hit_pos, 1.0);

    var output: FragmentOutput;
    output.depth = clip.z / clip.w;
    return output;
}
