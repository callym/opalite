#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 color;

layout(set = 0, binding = 0) uniform Locals {
    float test;
};

layout(location = 0) out vec3 v_color;

void main() {
    v_color = vec3(color.r, 0.0, test);
    gl_Position = vec4(position, 1.0);
}
