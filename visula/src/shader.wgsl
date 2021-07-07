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
    [[builtin(position)]] proj_position: vec4<f32>;
    [[location(0)]] plane_coord: vec2<f32>;
    [[location(1)]] radius: f32;
    [[location(2)]] vertex_position: vec3<f32>;
    [[location(3)]] instance_position: vec3<f32>;
    [[location(4)]] instance_color: vec3<f32>;
};

[[stage(vertex)]]
fn vs_main(
    [[location(0)]] position: vec3<f32>,
    [[location(1)]] instance_position: vec3<f32>,
    [[location(2)]] instance_radius: f32,
    [[location(3)]] instance_color: vec3<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    //let viewMatrix: mat3x3<f32> = mat3x3<f32>(
    //    u_globals.view_matrix[0].x, u_globals.view_matrix[1].x, u_globals.view_matrix[2].x,
    //    u_globals.view_matrix[0].y, u_globals.view_matrix[1].y, u_globals.view_matrix[2].y,
    //    u_globals.view_matrix[0].z, u_globals.view_matrix[1].z, u_globals.view_matrix[2].z
    //);
    //let viewMatrix: mat3x3<f32> = mat3x3<f32>(
    //    vec3<f32>(0.0, 0.0, 0.0),
    //    vec3<f32>(0.0, 0.0, 0.0),
    //    vec3<f32>(0.0, 0.0, 0.0)
    //);
    let viewMatrix: mat3x3<f32> = mat3x3<f32>(
        (vec4<f32>(1.0, 0.0, 0.0, 0.0) * u_globals.view_matrix).xyz,
        (vec4<f32>(0.0, 1.0, 0.0, 0.0) * u_globals.view_matrix).xyz,
        (vec4<f32>(0.0, 0.0, 1.0, 0.0) * u_globals.view_matrix).xyz,
    );

    //let cameraRight: vec3<f32> = vec3<f32>(viewMatrix[0].x, viewMatrix[1].x, viewMatrix[2].x);
    //let cameraUp: vec3<f32> = vec3<f32>(viewMatrix[0].y, viewMatrix[1].y, viewMatrix[2].y);
    //let cameraView: vec3<f32> = vec3<f32>(viewMatrix[0].z, viewMatrix[1].z, viewMatrix[2].z);
    let cameraRight: vec3<f32> = vec3<f32>(1.0, 0.0, 0.0);
    let cameraUp: vec3<f32> = vec3<f32>(0.0, 1.0, 0.0);
    let cameraView: vec3<f32> = vec3<f32>(0.0, 0.0, 1.0);

    let view: vec3<f32> = normalize(instance_position - u_globals.camera_position.xyz);
    let right: vec3<f32> = normalize(cross(view, cameraUp));
    let up: vec3<f32> = normalize(cross(right, view));

    let transform: mat3x3<f32> = mat3x3<f32>(right, up, view);

    let vertexOffset: vec3<f32> = transform * position;

    let vertexPosition: vec3<f32> = vertexOffset + instance_position;

    out.proj_position = u_globals.transform * vec4<f32>(vertexPosition, 1.0);
    out.plane_coord = position.xy;
    out.radius = instance_radius;
    out.vertex_position = vertexPosition;
    out.instance_position = instance_position;
    out.instance_color = instance_color;

    return out;
}

// fragment shader
[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let rayDirection: vec3<f32> = normalize(in.vertex_position - u_globals.camera_position.xyz);
    let rayOrigin: vec3<f32> = in.vertex_position - in.instance_position;

    let radius: f32 = in.radius;

    let E: vec3<f32> = rayOrigin;
    let D: vec3<f32> = rayDirection;

    // Sphere equation
    //     x^2 + y^2 + z^2 = r^2
    // Ray equation is
    //     P(t) = E + t*D
    // We substitute ray into sphere equation to get
    //     (Ex + Dx * t)^2 + (Ey + Dy * t)^2 + (Ez + Dz * t)^2 = r^2
    // Collecting the elements gives
    //     (Ex * Ex) + (2.0 * Ex * Dx) * t + (Dx * Dx) * t^2 + ... = r^2
    // Resulting in a second order equation with the following terms:

    let r2: f32 = radius*radius;
    let a: f32 = dot(D, D);
    let b: f32 = 2.0 * dot(E, D);
    let c: f32 = dot(E, E) - r2;

    // discriminant of sphere equation
    let d: f32 = b*b - 4.0 * a*c;
    if(d < 0.0) {
        discard;
    }

    let sqrtd: f32 = sqrt(d);
    let t1: f32 = (-b - sqrtd)/(2.0*a);
    let t2: f32 = (-b + sqrtd)/(2.0*a);

    let t: f32 = min(t1, t2);

    let sphereIntersection: vec3<f32> = rayOrigin + t * rayDirection;

    let normal: vec3<f32> = normalize(sphereIntersection);
    let normalDotCamera: f32 = dot(normal, -normalize(rayDirection));

    let position: vec3<f32> = in.instance_position + sphereIntersection;

    let color: vec3<f32> = in.instance_color;
    let projectedPoint: vec4<f32> = u_globals.transform * vec4<f32>(position, 1.0);

    // TODO fix frag depth
    // gl_FragDepth = projectedPoint.z / projectedPoint.w;

    return vec4<f32>(color * normalDotCamera, 1.0);
}
