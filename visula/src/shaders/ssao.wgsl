struct SsaoParams {
    radius: f32,
    bias: f32,
    intensity: f32,
    kernel_size: u32,
};

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

@group(0) @binding(0)
var depth_texture: texture_depth_2d;

@group(0) @binding(1)
var normal_texture: texture_2d<f32>;

@group(0) @binding(2)
var noise_texture: texture_2d<f32>;

@group(0) @binding(3)
var<uniform> camera: Camera;

@group(0) @binding(4)
var<uniform> params: SsaoParams;

@group(0) @binding(5)
var<storage, read> kernel: array<vec4<f32>>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(vertex_index & 1u)) * 4.0 - 1.0;
    let y = f32(i32(vertex_index >> 1u)) * 4.0 - 1.0;
    return vec4<f32>(x, -y, 0.0, 1.0);
}

fn reconstruct_view_position(coord: vec2<i32>) -> vec3<f32> {
    let depth = textureLoad(depth_texture, coord, 0);
    let screen_size = camera.screen_size.xy;
    let ndc = vec2<f32>(
        (f32(coord.x) + 0.5) / screen_size.x * 2.0 - 1.0,
        1.0 - (f32(coord.y) + 0.5) / screen_size.y * 2.0,
    );
    let clip = vec4<f32>(ndc, depth, 1.0);
    let view_pos = camera.inverse_projection_matrix * clip;
    return view_pos.xyz / view_pos.w;
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) f32 {
    let coord = vec2<i32>(frag_coord.xy);
    let screen_size = camera.screen_size.xy;

    let depth = textureLoad(depth_texture, coord, 0);
    if depth >= 1.0 {
        return 1.0;
    }

    let view_pos = reconstruct_view_position(coord);

    let world_normal = textureLoad(normal_texture, coord, 0).xyz;
    if length(world_normal) < 0.01 {
        return 1.0;
    }
    let view_normal = normalize((camera.view_matrix * vec4<f32>(world_normal, 0.0)).xyz);

    let noise_coord = coord % vec2<i32>(4, 4);
    let random_vec = textureLoad(noise_texture, noise_coord, 0).xyz;

    let tangent = normalize(random_vec - view_normal * dot(random_vec, view_normal));
    let bitangent = cross(view_normal, tangent);
    let tbn = mat3x3<f32>(tangent, bitangent, view_normal);

    var occlusion = 0.0;
    let kernel_size = params.kernel_size;
    for (var i = 0u; i < kernel_size; i++) {
        let sample_dir = tbn * kernel[i].xyz;
        let sample_pos = view_pos + sample_dir * params.radius;

        let proj = camera.projection_matrix * vec4<f32>(sample_pos, 1.0);
        let proj_ndc = proj.xyz / proj.w;
        let sample_coord = vec2<i32>(
            i32((proj_ndc.x * 0.5 + 0.5) * screen_size.x),
            i32((1.0 - (proj_ndc.y * 0.5 + 0.5)) * screen_size.y),
        );

        if sample_coord.x < 0 || sample_coord.x >= i32(screen_size.x) ||
           sample_coord.y < 0 || sample_coord.y >= i32(screen_size.y) {
            continue;
        }

        let sample_depth_view = reconstruct_view_position(sample_coord).z;

        let range_check = smoothstep(0.0, 1.0, params.radius / abs(view_pos.z - sample_depth_view));
        if sample_depth_view >= sample_pos.z + params.bias {
            occlusion += 1.0 * range_check;
        }
    }

    let ao = 1.0 - (occlusion / f32(kernel_size)) * params.intensity;
    return clamp(ao, 0.0, 1.0);
}
