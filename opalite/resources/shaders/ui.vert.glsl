#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec2 position;
layout(location = 1) in vec4 color;

layout(set = 0, binding = 0) uniform Locals {
    mat4 proj_view;
};

layout(location = 0) out vec4 v_color;

void main() {
    v_color = color;
    gl_Position = vec4(position, 0.0, 1.0);
}
