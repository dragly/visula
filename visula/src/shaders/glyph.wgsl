struct Globals {
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
var<uniform> u_globals: Globals;

@group(2)
@binding(0)
var atlas: texture_2d<f32>;

@group(2)
@binding(1)
var atlas_sampler: sampler;

struct VertexOutput {
    @builtin(position) proj_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) texel: vec2<f32>,
};

struct Glyph {
    color: vec4<f32>,
    transform: mat4x4<f32>,
};

fn glyph_vertex(vertex: vec4<f32>, glyph: Glyph) -> VertexOutput {
    var output: VertexOutput;
    let world_position = glyph.transform * vec4<f32>(vertex.xy, 0.0, 1.0);
    output.proj_position = u_globals.transform * world_position;
    output.color = glyph.color;
    output.texel = vertex.zw;
    return output;
}

@vertex
fn vs_main(
    @location(0) vertex: vec4<f32>,
) -> VertexOutput {
    var glyph: Glyph;
    return glyph_vertex(vertex, glyph);
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) normal: vec4<f32>,
};

fn median(a: f32, b: f32, c: f32) -> f32 {
    return max(min(a, b), min(max(a, b), c));
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let size = vec2<f32>(textureDimensions(atlas));
    let field = textureSample(atlas, atlas_sampler, in.texel / size).rgb;
    let texels_per_pixel = fwidth(in.texel);
    let pixels_per_texel = 2.0 / (texels_per_pixel.x + texels_per_pixel.y);
    let pixel_range = max(DISTANCE_RANGE * pixels_per_texel, 1.0);
    let distance = median(field.r, field.g, field.b) - 0.5;
    let coverage = clamp(distance * pixel_range + 0.5, 0.0, 1.0);
    let luminance = dot(in.color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let alpha = in.color.a * pow(coverage, mix(1.0 / 2.2, 2.2, luminance));
    if alpha < 0.01 {
        discard;
    }

    var output: FragmentOutput;
    output.color = vec4<f32>(in.color.rgb, alpha);
    output.normal = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    return output;
}
