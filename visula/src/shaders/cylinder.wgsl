struct Camera {
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
var<uniform> u_globals: Camera;

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

struct CylinderMaterial {
    color: vec3<f32>,
};

fn cylinders(
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

    output.proj_position = u_globals.transform * vec4<f32>(world_pos, 1.0);
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
    return cylinders(vertex_position, cylinder_geometry);
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

fn intersect_cone(
    ray_origin: vec3<f32>,
    ray_dir: vec3<f32>,
    cyl_start: vec3<f32>,
    cyl_end: vec3<f32>,
    start_radius: f32,
    end_radius: f32,
) -> vec4<f32> {
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

    // Use large sentinel value; any real intersection will be smaller
    var best_t = 1e30;
    var best_normal = vec3<f32>(0.0);

    if discriminant >= 0.0 {
        let sqrtd = sqrt(discriminant);
        let t1 = (-b - sqrtd) / (2.0 * a);
        let t2 = (-b + sqrtd) / (2.0 * a);

        for (var i = 0; i < 2; i++) {
            let t = select(t2, t1, i == 0);
            let p = oc + t * ray_dir;
            let h = dot(p, dir);
            if h >= 0.0 && h <= height && t > 0.0 {
                if t < best_t {
                    best_t = t;
                    let r = start_radius + slope * h;
                    let p_on_axis = h * dir;
                    let radial = normalize(p - p_on_axis);
                    let tangent_angle = atan2(dr, height);
                    best_normal = normalize(radial * cos(tangent_angle) + dir * (-sin(tangent_angle)));
                }
            }
        }
    }

    // End caps
    for (var cap = 0; cap < 2; cap++) {
        let cap_h = select(0.0, height, cap == 1);
        let cap_r = select(start_radius, end_radius, cap == 1);
        let cap_normal_dir = select(-dir, dir, cap == 1);

        let denom = dot(ray_dir, cap_normal_dir);
        if abs(denom) < 0.0001 {
            continue;
        }
        let cap_center = cyl_start + cap_h * dir;
        let t = dot(cap_center - ray_origin, cap_normal_dir) / denom;
        let p = ray_origin + t * ray_dir - cap_center;
        if dot(p, p) <= cap_r * cap_r && t > 0.0 {
            if t < best_t {
                best_t = t;
                best_normal = cap_normal_dir;
            }
        }
    }

    if best_t > 1e29 {
        return vec4<f32>(-1e30, 0.0, 0.0, 0.0);
    }

    return vec4<f32>(best_t, best_normal);
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let ray_origin = in.vertex_position;
    let ray_direction = normalize(in.vertex_position - u_globals.camera_position.xyz);

    let result = intersect_cone(ray_origin, ray_direction, in.cyl_start, in.cyl_end, in.start_radius, in.end_radius);
    let t = result.x;

    if t < 0.0 || t > 1e29 {
        discard;
    }

    let hit_pos = ray_origin + t * ray_direction;

    var _visula_normal: vec3<f32> = normalize(result.yzw);
    var _visula_position: vec3<f32> = hit_pos;
    var _visula_view_direction: vec3<f32> = -ray_direction;
    var _visula_input_color: vec3<f32> = in.input_color;

    let clip_position = u_globals.transform * vec4<f32>(hit_pos, 1.0);
    let frag_depth = clip_position.z / clip_position.w;

    var cylinder_material: CylinderMaterial;

    var output: FragmentOutput;
    output.color = vec4<f32>(cylinder_material.color, 1.0);
    output.normal = vec4<f32>(_visula_normal, 0.0);
    output.depth = frag_depth;
    return output;
}
