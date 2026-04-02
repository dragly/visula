struct Globals {
    view_matrix: mat4x4<f32>,
    model_view_projection_matrix: mat4x4<f32>,
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

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world_position: vec3<f32>,
    @location(3) vertex_color: vec4<f32>,
};

struct MeshGeometry {
    rotation: vec4<f32>,
    position: vec3<f32>,
    scale: vec3<f32>,
};

struct MeshMaterial {
    color: vec4<f32>,
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
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
) -> VertexOutput {
    var geometry: MeshGeometry;
    var out: VertexOutput;
    let transform_matrix = calculate_transform_matrix(geometry.rotation, geometry.position, geometry.scale);
    out.position = u_globals.model_view_projection_matrix * transform_matrix * vec4<f32>(position, 1.0);
    let normal_matrix = mat3x3(transform_matrix[0].xyz, transform_matrix[1].xyz, transform_matrix[2].xyz);
    out.normal = normal_matrix * normal;
    out.uv = uv;
    out.world_position = (transform_matrix * vec4<f32>(position, 1.0)).xyz;
    out.vertex_color = color;
    return out;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) normal: vec4<f32>,
};

@fragment
fn fs_main(vertex: VertexOutput) -> FragmentOutput {
    var _visula_normal: vec3<f32> = normalize(vertex.normal);
    var _visula_position: vec3<f32> = vertex.world_position;
    var _visula_view_direction: vec3<f32> = normalize(u_globals.camera_position.xyz - vertex.world_position);
    var _visula_input_color: vec4<f32> = vertex.vertex_color;

    var material: MeshMaterial;

    var output: FragmentOutput;
    output.color = vec4<f32>(material.color.xyz, 1.0);
    output.normal = vec4<f32>(_visula_normal, 0.0);
    return output;
}
