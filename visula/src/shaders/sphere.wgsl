struct Camera {
    view_matrix: mat4x4<f32>,
    transform: mat4x4<f32>,
    camera_center: vec4<f32>,
    camera_view_vector: vec4<f32>,
    camera_position: vec4<f32>,
    camera_up: vec4<f32>,
};

@group(0)
@binding(0)
var<uniform> u_globals: Camera;

struct VertexOutput {
    @builtin(position) proj_position: vec4<f32>,
    @location(0) plane_coord: vec2<f32>,
    @location(1) radius: f32,
    @location(2) vertex_position: vec3<f32>,
    @location(3) instance_position: vec3<f32>,
    @location(4) instance_color: vec3<f32>,
};

struct SphereGeometry {
    position: vec3<f32>,
    radius: f32,
    color: vec3<f32>,
};

struct SphereMaterial {
    color: vec3<f32>,
};

fn spheres(
    vertex_offset_pre_transform: vec4<f32>,
    sphere: SphereGeometry,
) -> VertexOutput {
    var output: VertexOutput;
    let viewMatrix: mat3x3<f32> = mat3x3<f32>(
        (vec4<f32>(1.0, 0.0, 0.0, 0.0) * u_globals.view_matrix).xyz,
        (vec4<f32>(0.0, 1.0, 0.0, 0.0) * u_globals.view_matrix).xyz,
        (vec4<f32>(0.0, 0.0, 1.0, 0.0) * u_globals.view_matrix).xyz,
    );

    let cameraRight: vec3<f32> = vec3<f32>(1.0, 0.0, 0.0);
    let cameraUp: vec3<f32> = vec3<f32>(0.0, 1.0, 0.0);
    let cameraView: vec3<f32> = vec3<f32>(0.0, 0.0, 1.0);

    let view: vec3<f32> = normalize(sphere.position - u_globals.camera_position.xyz);
    let right: vec3<f32> = normalize(cross(view, cameraUp));
    let up: vec3<f32> = normalize(cross(right, view));

    let transform: mat3x3<f32> = mat3x3<f32>(right, up, view);

    let vertexOffset: vec3<f32> = sphere.radius * (transform * vertex_offset_pre_transform.xyz);

    let vertexPosition: vec3<f32> = vertexOffset + sphere.position;

    output.proj_position = u_globals.transform * vec4<f32>(vertexPosition, 1.0);
    output.plane_coord = vertex_offset_pre_transform.xy;
    output.radius = sphere.radius;
    output.vertex_position = vertexPosition;
    output.instance_position = sphere.position;
    output.instance_color = sphere.color;

    return output;
}

@vertex
fn vs_main(
    @location(0) vertex_offset_pre_transform: vec4<f32>,
) -> VertexOutput {
    var sphere_geometry: SphereGeometry;
    return spheres(vertex_offset_pre_transform, sphere_geometry);
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let rayDirection: vec3<f32> = normalize(in.vertex_position - u_globals.camera_position.xyz);
    let rayOrigin: vec3<f32> = in.vertex_position - in.instance_position;

    let radius: f32 = in.radius;

    let E: vec3<f32> = rayOrigin;
    let D: vec3<f32> = rayDirection;

    let r2: f32 = radius*radius;
    let a: f32 = dot(D, D);
    let b: f32 = 2.0 * dot(E, D);
    let c: f32 = dot(E, E) - r2;

    let d: f32 = b*b - 4.0 * a*c;
    if(d < 0.0) {
        discard;
    }

    let sqrtd: f32 = sqrt(d);
    let t1: f32 = (-b - sqrtd)/(2.0*a);
    let t2: f32 = (-b + sqrtd)/(2.0*a);

    let t: f32 = min(t1, t2);

    let sphereIntersection: vec3<f32> = rayOrigin + t * rayDirection;

    var _visula_normal: vec3<f32> = normalize(sphereIntersection);
    var _visula_position: vec3<f32> = in.instance_position + sphereIntersection;
    var _visula_view_direction: vec3<f32> = -rayDirection;
    var _visula_instance_color: vec3<f32> = in.instance_color;

    let clip_position: vec4<f32> = u_globals.transform * vec4<f32>(_visula_position, 1.0);
    let frag_depth: f32 = clip_position.z / clip_position.w;

    var sphere_material: SphereMaterial;

    var output: FragmentOutput;
    output.color = vec4<f32>(sphere_material.color, 1.0);
    output.depth = frag_depth;
    return output;
}
