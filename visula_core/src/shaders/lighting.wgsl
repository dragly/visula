fn visula_lit_vec3(color: vec3<f32>, normal: vec3<f32>, view_direction: vec3<f32>) -> vec3<f32> {
    let sun1 = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let sun2 = normalize(vec3<f32>(-1.0, -0.8, -0.4));
    let intensity = clamp(
        dot(normal, view_direction) + dot(normal, sun1) + dot(normal, sun2),
        0.05,
        1.0,
    );
    return color * intensity;
}

fn visula_lit_vec4(color: vec4<f32>, normal: vec3<f32>, view_direction: vec3<f32>) -> vec4<f32> {
    return vec4<f32>(visula_lit_vec3(color.xyz, normal, view_direction), color.w);
}

fn visula_directional_lit_vec3(color: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let sun1 = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let sun2 = normalize(vec3<f32>(-1.0, -0.8, -0.4));
    let intensity = clamp(
        dot(normal, sun1) + dot(normal, sun2),
        0.05,
        1.0,
    );
    return color * intensity;
}

fn visula_directional_lit_vec4(color: vec4<f32>, normal: vec3<f32>) -> vec4<f32> {
    return vec4<f32>(visula_directional_lit_vec3(color.xyz, normal), color.w);
}
