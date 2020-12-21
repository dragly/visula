#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec4 colour;

layout(location = 0) out vec4 v_Colour;

layout(set = 0, binding = 0) uniform Locals {
    mat4 u_ViewMatrix;
    mat4 u_Transform;
    vec3 u_CameraCenter;
    vec3 u_CameraViewVector;
    vec3 u_CameraPosition;
    vec3 u_CameraUp;
};

void main() {
    gl_Position = u_Transform * vec4(position, 1.0);
    v_Colour = colour;
}
