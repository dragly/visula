#version 450

layout(location = 0) in vec3 a_Pos;
layout(location = 1) in vec3 a_InstancePos;
layout(location = 2) in float a_InstanceRadius;
layout(location = 3) in vec3 a_InstanceColor;

layout(location = 0) out vec2 v_PlaneCoord;
layout(location = 1) out float v_Radius;
layout(location = 2) out vec3 v_VertexPosition;
layout(location = 3) out vec3 v_InstancePosition;
layout(location = 4) out vec3 v_InstanceColor;

layout(set = 0, binding = 0) uniform Locals {
    mat4 u_ViewMatrix;
    mat4 u_Transform;
    vec3 u_CameraCenter;
    vec3 u_CameraViewVector;
    vec3 u_CameraPosition;
    vec3 u_CameraUp;
};

void main() {
    mat3 viewMatrix = mat3(u_ViewMatrix);

    vec3 cameraRight = vec3(viewMatrix[0][0], viewMatrix[1][0], viewMatrix[2][0]);
    vec3 cameraUp = vec3(viewMatrix[0][1], viewMatrix[1][1], viewMatrix[2][1]);
    vec3 cameraView = vec3(viewMatrix[0][2], viewMatrix[1][2], viewMatrix[2][2]);

    vec3 view = normalize(a_InstancePos - u_CameraPosition);
    vec3 right = normalize(cross(view, cameraUp));
    vec3 up = normalize(cross(right, view));

    mat3 transform = 1.0 * mat3(right, up, view);

    vec3 vertexOffset = transform * a_Pos;

    vec3 vertexPosition = vertexOffset + a_InstancePos;

    v_PlaneCoord = a_Pos.xy;
    v_Radius = a_InstanceRadius;
    v_VertexPosition = vertexPosition;
    v_InstancePosition = a_InstancePos;
    v_InstanceColor = a_InstanceColor;

    gl_Position = u_Transform * vec4(vertexPosition, 1.0);
}
