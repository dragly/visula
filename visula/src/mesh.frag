#version 450

layout(early_fragment_tests) in;

layout(location = 0) in vec4 v_Colour;

layout(location = 0) out vec4 outColour;

void main() {
    outColour = v_Colour;
    outColour.a = 1.0;
}
