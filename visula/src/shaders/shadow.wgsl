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
};

struct ShadowGeometry {
    position: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) vertex_position: vec4<f32>,
) -> VertexOutput {
    var shadow_geometry: ShadowGeometry;
    var output: VertexOutput;
    output.proj_position = u_light.light_view_proj * vec4<f32>(shadow_geometry.position, 1.0);
    return output;
}
