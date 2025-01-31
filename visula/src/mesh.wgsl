struct Globals {
    view_matrix: mat4x4<f32>,
    model_view_projection_matrix: mat4x4<f32>,
    camera_center: vec4<f32>,
    camera_view_vector: vec4<f32>,
    camera_position: vec4<f32>,
    camera_up: vec4<f32>,
};

@group(0)
@binding(0)
var<uniform> u_globals: Globals;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct MeshInstance {
    rotation: vec4<f32>,
    position: vec3<f32>,
    scale: vec3<f32>,
};

fn calculate_transform_matrix(
    rotation: vec4<f32>,
    translation: vec3<f32>,
    scale: vec3<f32>,
) -> mat4x4<f32> {
    let x = rotation.x;
    let y = rotation.y;
    let z = rotation.z;
    let w = rotation.w;
    let x2 = x + x;
    let y2 = y + y;
    let z2 = z + z;
    let xx = x * x2;
    let xy = x * y2;
    let xz = x * z2;
    let yy = y * y2;
    let yz = y * z2;
    let zz = z * z2;
    let wx = w * x2;
    let wy = w * y2;
    let wz = w * z2;

    let x_axis = vec4<f32>(1.0 - (yy + zz), xy + wz, xz - wy, 0.0);
    let y_axis = vec4<f32>(xy - wz, 1.0 - (xx + zz), yz + wx, 0.0);
    let z_axis = vec4<f32>(xz + wy, yz - wx, 1.0 - (xx + yy), 0.0);
    return mat4x4(scale.x * x_axis, scale.y * y_axis, scale.z * z_axis, vec4(translation, 1.0));
}

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
) -> VertexOutput {
    var instance: MeshInstance;
    // modification happens here
    var out: VertexOutput;
    let transform_matrix = calculate_transform_matrix(instance.rotation, instance.position, instance.scale);
    out.position = u_globals.model_view_projection_matrix * transform_matrix * vec4<f32>(position, 1.0);
    let normal_matrix = mat3x3(transform_matrix[0].xyz, transform_matrix[1].xyz, transform_matrix[2].xyz); // TODO figure out how to get hold of inverse
    out.normal = normal_matrix * normal;
    out.color = color;
    return out;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let rayDirection: vec3<f32> = normalize(vertex.position.xyz - u_globals.camera_position.xyz);
    let sun1 = vec3<f32>(-1.0, 1.0, -1.0);
    let sun2 = vec3<f32>(1.0, 1.0, 1.0);
    let normal: vec3<f32> = normalize(vertex.normal);
    let normalDotCamera: f32 = dot(normal, normalize(rayDirection));
    let normalDotSun1: f32 = dot(normal, normalize(sun1));
    let normalDotSun2: f32 = dot(normal, normalize(sun2));
    let intensity = (
        0.5 * clamp(normalDotCamera, 0.0, 1.0) +
        0.5 * clamp(normalDotSun1, 0.0, 1.0) +
        0.5 * clamp(normalDotSun2, 0.0, 1.0)
    );
    return vec4<f32>(vertex.color.xyz * clamp(intensity, 0.0, 1.0), 1.0);
}
