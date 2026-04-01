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

struct SkyParams {
    mode: u32,
};

@group(0) @binding(0)
var<uniform> u_globals: Camera;

@group(1) @binding(0)
var<uniform> sky_params: SkyParams;

struct FullscreenOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> FullscreenOutput {
    let x = f32(i32(vertex_index & 1u)) * 4.0 - 1.0;
    let y = f32(i32(vertex_index >> 1u)) * 4.0 - 1.0;
    var output: FullscreenOutput;
    output.position = vec4<f32>(x, -y, 1.0, 1.0);
    output.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (-y * 0.5 + 0.5));
    return output;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) normal: vec4<f32>,
};

@fragment
fn fs_main(in: FullscreenOutput) -> FragmentOutput {
    let ndc = vec2<f32>(
        (in.position.x / u_globals.screen_size.x) * 2.0 - 1.0,
        1.0 - (in.position.y / u_globals.screen_size.y) * 2.0,
    );
    let clip = vec4<f32>(ndc, 1.0, 1.0);
    let world_pos = u_globals.inverse_view_proj * clip;
    let world_dir = normalize(world_pos.xyz / world_pos.w - u_globals.camera_position.xyz);

    var sky_color: vec3<f32>;

    if sky_params.mode == 1u {
        // Normal map mode
        sky_color = (world_dir * 0.5 + 0.5) * 0.3;
    } else if sky_params.mode == 2u {
        // Sky / Ground mode
        let up_factor = world_dir.y * 0.5 + 0.5;
        let sky_top = vec3<f32>(0.4, 0.6, 0.9);
        let sky_horizon = vec3<f32>(0.7, 0.8, 0.95);
        let ground_horizon = vec3<f32>(0.5, 0.45, 0.4);
        let ground_bottom = vec3<f32>(0.3, 0.25, 0.2);
        if world_dir.y >= 0.0 {
            let t = pow(world_dir.y, 0.5);
            sky_color = mix(sky_horizon, sky_top, t);
        } else {
            let t = pow(-world_dir.y, 0.5);
            sky_color = mix(ground_horizon, ground_bottom, t);
        }
    } else {
        sky_color = vec3<f32>(0.0, 0.0, 0.0);
    }

    var output: FragmentOutput;
    output.color = vec4<f32>(sky_color, 1.0);
    output.normal = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    return output;
}
