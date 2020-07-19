#version 450

layout(location = 0) in vec2 v_PlaneCoord;
layout(location = 1) in float v_Radius;
layout(location = 2) in vec3 v_VertexPosition;
layout(location = 3) in vec3 v_InstancePosition;
layout(location = 4) in vec3 v_InstanceColor;

layout(location = 0) out vec4 outColor;

layout(set = 0, binding = 0) uniform Locals {
    mat4 u_ViewMatrix;
    mat4 u_Transform;
    vec3 u_CameraCenter;
    vec3 u_CameraViewVector;
    vec3 u_CameraPosition;
    vec3 u_CameraUp;
};

void main() {
    vec3 rayDirection = normalize(v_VertexPosition - u_CameraPosition);
    vec3 rayOrigin = v_VertexPosition - v_InstancePosition;

    float radius = v_Radius;

    vec3 E = rayOrigin;
    vec3 D = rayDirection;

    // Sphere equation
    //     x^2 + y^2 + z^2 = r^2
    // Ray equation is
    //     P(t) = E + t*D
    // We substitute ray into sphere equation to get
    //     (Ex + Dx * t)^2 + (Ey + Dy * t)^2 + (Ez + Dz * t)^2 = r^2
    // Collecting the elements gives
    //     (Ex * Ex) + (2.0 * Ex * Dx) * t + (Dx * Dx) * t^2 + ... = r^2
    // Resulting in a second order equation with the following terms:

    float r2 = radius*radius;
    float a = dot(D, D);
    float b = 2.0 * dot(E, D);
    float c = dot(E, E) - r2;

    // discriminant of sphere equation
    float d = b*b - 4.0 * a*c;
    if(d < 0.0) {
        discard;
    }

    float sqrtd = sqrt(d);
    float t1 = (-b - sqrtd)/(2.0*a);
    float t2 = (-b + sqrtd)/(2.0*a);

    float t = min(t1, t2);

    vec3 sphereIntersection = rayOrigin + t * rayDirection;

    vec3 normal = normalize(sphereIntersection);
    float normalDotCamera = dot(normal, -normalize(rayDirection));

    vec3 position = v_InstancePosition + sphereIntersection;

    vec3 color = v_InstanceColor;
    outColor = vec4(color * normalDotCamera, 1.0);
    vec4 projectedPoint = u_Transform * vec4(position, 1.0);

    gl_FragDepth = projectedPoint.z / projectedPoint.w;
}
