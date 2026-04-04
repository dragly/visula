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
    @location(1) torus_center: vec3<f32>,
    @location(2) torus_params: vec2<f32>,
    @location(3) input_color: vec3<f32>,
    @location(4) torus_up: vec3<f32>,
    @location(5) torus_right: vec3<f32>,
    @location(6) torus_forward: vec3<f32>,
};

struct TorusGeometry {
    position: vec3<f32>,
    major_radius: f32,
    minor_radius: f32,
    rotation: vec4<f32>,
    color: vec3<f32>,
};

struct TorusMaterial {
    color: vec3<f32>,
};

fn quat_rotate(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    let u = q.xyz;
    let s = q.w;
    return 2.0 * dot(u, v) * u + (s * s - dot(u, u)) * v + 2.0 * s * cross(u, v);
}

fn torus_vertex(
    vertex_position: vec3<f32>,
    torus: TorusGeometry,
) -> VertexOutput {
    var output: VertexOutput;

    let right = quat_rotate(torus.rotation, vec3<f32>(1.0, 0.0, 0.0));
    let up = quat_rotate(torus.rotation, vec3<f32>(0.0, 1.0, 0.0));
    let forward = quat_rotate(torus.rotation, vec3<f32>(0.0, 0.0, 1.0));

    let extent = torus.major_radius + torus.minor_radius;
    let padding = torus.minor_radius * 0.5;

    let world_pos = torus.position
        + right * vertex_position.x * (extent + padding)
        + up * vertex_position.y * (torus.minor_radius + padding)
        + forward * vertex_position.z * (extent + padding);

    output.proj_position = u_globals.transform * vec4<f32>(world_pos, 1.0);
    output.vertex_position = world_pos;
    output.torus_center = torus.position;
    output.torus_params = vec2<f32>(torus.major_radius, torus.minor_radius);
    output.input_color = torus.color;
    output.torus_up = up;
    output.torus_right = right;
    output.torus_forward = forward;

    return output;
}

@vertex
fn vs_main(
    @location(0) vertex_position: vec3<f32>,
) -> VertexOutput {
    var torus_geometry: TorusGeometry;
    return torus_vertex(vertex_position, torus_geometry);
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

// Solve depressed cubic t^3 + pt + q = 0, return one real root
fn solve_cubic_one(p: f32, q: f32) -> f32 {
    let d = q * q / 4.0 + p * p * p / 27.0;
    if d >= 0.0 {
        let sd = sqrt(d);
        let u = sign(-q / 2.0 + sd) * pow(abs(-q / 2.0 + sd), 1.0 / 3.0);
        let v = sign(-q / 2.0 - sd) * pow(abs(-q / 2.0 - sd), 1.0 / 3.0);
        return u + v;
    } else {
        let r = sqrt(-p * p * p / 27.0);
        let phi = acos(clamp(-q / (2.0 * r), -1.0, 1.0));
        return 2.0 * pow(r, 1.0 / 3.0) * cos(phi / 3.0);
    }
}

fn min_positive(a: f32, b: f32) -> f32 {
    if b > 0.0 && b < a {
        return b;
    }
    return a;
}

// Solve quartic x^4 + bx^3 + cx^2 + dx + e = 0 using Ferrari's method
// Returns the smallest positive real root, or 1e30 if none
fn solve_quartic(b: f32, c: f32, d: f32, e: f32) -> f32 {
    let b2 = b * b;
    let b3 = b2 * b;
    let b4 = b2 * b2;

    // Depress: x = t - b/4 gives t^4 + pt^2 + qt + r = 0
    let p = c - 3.0 * b2 / 8.0;
    let q = d - b * c / 2.0 + b3 / 8.0;
    let r = e - b * d / 4.0 + b2 * c / 16.0 - 3.0 * b4 / 256.0;

    if abs(q) < 1e-10 {
        let disc = p * p - 4.0 * r;
        if disc < 0.0 { return 1e30; }
        let sd = sqrt(disc);
        var best = 1e30;
        let z1 = (-p + sd) / 2.0;
        let z2 = (-p - sd) / 2.0;
        if z1 >= 0.0 {
            let v = sqrt(z1);
            best = min_positive(best, v - b / 4.0);
            best = min_positive(best, -v - b / 4.0);
        }
        if z2 >= 0.0 {
            let v = sqrt(z2);
            best = min_positive(best, v - b / 4.0);
            best = min_positive(best, -v - b / 4.0);
        }
        return best;
    }

    // Resolvent cubic: m^3 - (p/2)*m^2 - r*m + (4pr - q^2)/8 = 0
    // Depress with m = u + p/6:
    let hp = -p / 2.0;
    let rc = -r;
    let rd = (4.0 * p * r - q * q) / 8.0;
    let cp = rc - hp * hp / 3.0;
    let cq = rd - hp * rc / 3.0 + 2.0 * hp * hp * hp / 27.0;
    var m = solve_cubic_one(cp, cq) - hp / 3.0;

    let disc_check = 2.0 * m - p;
    if disc_check < 0.0 {
        m = m + abs(disc_check) + 1e-6;
    }

    // Factor: (t^2 - s*t + (m + q/(2s))) * (t^2 + s*t + (m - q/(2s))) = 0
    let s = sqrt(2.0 * m - p);
    let q_over_2s = q / (2.0 * s);

    let d1 = s * s - 4.0 * (m + q_over_2s);
    let d2 = s * s - 4.0 * (m - q_over_2s);

    var best = 1e30;

    if d1 >= 0.0 {
        let sd1 = sqrt(d1);
        best = min_positive(best, (s + sd1) / 2.0 - b / 4.0);
        best = min_positive(best, (s - sd1) / 2.0 - b / 4.0);
    }
    if d2 >= 0.0 {
        let sd2 = sqrt(d2);
        best = min_positive(best, (-s + sd2) / 2.0 - b / 4.0);
        best = min_positive(best, (-s - sd2) / 2.0 - b / 4.0);
    }

    return best;
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let ray_origin_world = in.vertex_position;
    let ray_dir_world = normalize(in.vertex_position - u_globals.camera_position.xyz);

    // Transform to torus local space (torus lies in XZ plane, Y is up)
    let oc = ray_origin_world - in.torus_center;
    let local_origin = vec3<f32>(
        dot(oc, in.torus_right),
        dot(oc, in.torus_up),
        dot(oc, in.torus_forward),
    );
    let local_dir = vec3<f32>(
        dot(ray_dir_world, in.torus_right),
        dot(ray_dir_world, in.torus_up),
        dot(ray_dir_world, in.torus_forward),
    );

    let R = in.torus_params.x;
    let r = in.torus_params.y;
    let R2 = R * R;
    let r2 = r * r;

    let o = local_origin;
    let d = local_dir;

    let od = dot(o, d);
    let oo = dot(o, o);
    let dd = dot(d, d);
    let k = oo - R2 - r2;

    // Quartic from torus implicit: (x^2+y^2+z^2 + R^2 - r^2)^2 = 4R^2(x^2+z^2)
    // Substituting P = o + t*d:
    let a4 = dd * dd;
    let a3 = 4.0 * dd * od;
    let a2 = 2.0 * dd * k + 4.0 * od * od + 4.0 * R2 * d.y * d.y;
    let a1 = 4.0 * od * k + 8.0 * R2 * o.y * d.y;
    let a0 = k * k + 4.0 * R2 * o.y * o.y - 4.0 * R2 * r2;

    // Normalize to monic
    let inv_a4 = 1.0 / a4;
    var t = solve_quartic(a3 * inv_a4, a2 * inv_a4, a1 * inv_a4, a0 * inv_a4);

    if t < 0.0 || t > 1e29 {
        discard;
    }

    // Refine with Newton iterations for numerical stability
    for (var i = 0; i < 3; i++) {
        let P = local_origin + t * local_dir;
        let sum_sq = dot(P, P);
        let f = sum_sq * sum_sq - 2.0 * (R2 + r2) * sum_sq + 4.0 * R2 * P.y * P.y + (R2 - r2) * (R2 - r2);
        let g = dot(P, local_dir);
        let fp = 4.0 * sum_sq * g - 4.0 * (R2 + r2) * g + 8.0 * R2 * P.y * local_dir.y;
        if abs(fp) > 1e-10 {
            t = t - f / fp;
        }
    }

    // Verify the hit is close to the torus surface
    let P_verify = local_origin + t * local_dir;
    let sum_sq_v = dot(P_verify, P_verify);
    let torus_val = sum_sq_v * sum_sq_v - 2.0 * (R2 + r2) * sum_sq_v + 4.0 * R2 * P_verify.y * P_verify.y + (R2 - r2) * (R2 - r2);
    if abs(torus_val) > r2 * 0.5 || t < 0.0 {
        discard;
    }

    let local_hit = local_origin + t * local_dir;
    let hit_pos = ray_origin_world + t * ray_dir_world;

    // Normal: project to nearest point on the ring, normal = hit - ring_point
    let xz_len = length(vec2<f32>(local_hit.x, local_hit.z));
    let ring_point = vec3<f32>(
        local_hit.x * R / max(xz_len, 0.0001),
        0.0,
        local_hit.z * R / max(xz_len, 0.0001),
    );
    let local_n = normalize(local_hit - ring_point);

    // Transform normal back to world space
    var _visula_normal = normalize(
        local_n.x * in.torus_right +
        local_n.y * in.torus_up +
        local_n.z * in.torus_forward
    );
    var _visula_position = hit_pos;
    var _visula_view_direction = -ray_dir_world;
    var _visula_input_color = in.input_color;

    let clip_position = u_globals.transform * vec4<f32>(hit_pos, 1.0);
    let frag_depth = clip_position.z / clip_position.w;

    var torus_material: TorusMaterial;

    var output: FragmentOutput;
    output.color = vec4<f32>(torus_material.color, 1.0);
    output.normal = vec4<f32>(_visula_normal, 0.0);
    output.depth = frag_depth;
    return output;
}
