struct Light {
    direction: vec3<f32>,
    _pad0: f32,
    color: vec3<f32>,
    intensity: f32,
    light_view_proj: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> u_light: Light;

@group(1) @binding(1)
var shadow_map: texture_depth_2d;

@group(1) @binding(2)
var shadow_sampler: sampler_comparison;

fn compute_shadow(world_position: vec3<f32>) -> f32 {
    let light_clip = u_light.light_view_proj * vec4<f32>(world_position, 1.0);
    let light_ndc = light_clip.xyz / light_clip.w;
    let shadow_uv = light_ndc.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);

    if shadow_uv.x < 0.0 || shadow_uv.x > 1.0 || shadow_uv.y < 0.0 || shadow_uv.y > 1.0 {
        return 1.0;
    }

    let depth = light_ndc.z;

    // Slope-scaled bias: steeper surfaces need more bias to avoid acne.
    let light_dir = normalize(-u_light.direction);
    let dx = dpdx(depth);
    let dy = dpdy(depth);
    let slope = sqrt(dx * dx + dy * dy);
    let bias = max(0.0005, 0.002 * slope);

    let texel_size = 3.0 / 4096.0;
    var shadow = 0.0;
    for (var y = -2; y <= 2; y++) {
        for (var x = -2; x <= 2; x++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            shadow += textureSampleCompare(shadow_map, shadow_sampler, shadow_uv + offset, depth - bias);
        }
    }
    return shadow / 25.0;
}

fn visula_lit_vec3(color: vec3<f32>, normal: vec3<f32>, view_direction: vec3<f32>, world_position: vec3<f32>) -> vec3<f32> {
    let shadow = compute_shadow(world_position);

    let light_dir = normalize(-u_light.direction);
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let main_diffuse = n_dot_l * shadow * u_light.intensity;
    let half_dir = normalize(light_dir + normalize(view_direction));
    let main_specular = pow(max(dot(normal, half_dir), 0.0), 32.0) * shadow;

    let fill_dir = normalize(vec3<f32>(1.0, 0.4, -0.8));
    let fill_intensity = 0.3;
    let fill_diffuse = max(dot(normal, fill_dir), 0.0) * fill_intensity;

    let ambient = 0.1;

    return color * (ambient + main_diffuse + fill_diffuse) * u_light.color + main_specular * 0.3 * u_light.color;
}

fn visula_lit_vec4(color: vec4<f32>, normal: vec3<f32>, view_direction: vec3<f32>, world_position: vec3<f32>) -> vec4<f32> {
    return vec4<f32>(visula_lit_vec3(color.xyz, normal, view_direction, world_position), color.w);
}

fn visula_directional_lit_vec3(color: vec3<f32>, normal: vec3<f32>, world_position: vec3<f32>) -> vec3<f32> {
    let shadow = compute_shadow(world_position);

    let light_dir = normalize(-u_light.direction);
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let main_diffuse = n_dot_l * shadow * u_light.intensity;

    let fill_dir = normalize(vec3<f32>(1.0, 0.4, -0.8));
    let fill_intensity = 0.3;
    let fill_diffuse = max(dot(normal, fill_dir), 0.0) * fill_intensity;

    let ambient = 0.1;

    return color * (ambient + main_diffuse + fill_diffuse) * u_light.color;
}

fn visula_directional_lit_vec4(color: vec4<f32>, normal: vec3<f32>, world_position: vec3<f32>) -> vec4<f32> {
    return vec4<f32>(visula_directional_lit_vec3(color.xyz, normal, world_position), color.w);
}

fn visula_toon_lit_vec3(color: vec3<f32>, normal: vec3<f32>, view_direction: vec3<f32>, world_position: vec3<f32>) -> vec3<f32> {
    let shadow = compute_shadow(world_position);

    let light_dir = normalize(-u_light.direction);
    let n_dot_l = dot(normal, light_dir);
    let shade = smoothstep(-0.05, 0.05, n_dot_l) * shadow;
    let shadow_color = color * 0.5;
    var col = mix(shadow_color, color, shade);

    let half_dir = normalize(light_dir + normalize(view_direction));
    let spec = pow(max(dot(normal, half_dir), 0.0), 10.0);
    let highlight = mix(color, vec3<f32>(1.0, 1.0, 1.0), 0.5);
    col = mix(col, highlight, smoothstep(0.42, 0.48, spec) * 0.65);

    let fresnel = pow(1.0 - max(dot(normalize(view_direction), normal), 0.0), 2.5);
    col = col + color * fresnel * 0.6;

    return col;
}

fn visula_toon_lit_vec4(color: vec4<f32>, normal: vec3<f32>, view_direction: vec3<f32>, world_position: vec3<f32>) -> vec4<f32> {
    return vec4<f32>(visula_toon_lit_vec3(color.xyz, normal, view_direction, world_position), color.w);
}
