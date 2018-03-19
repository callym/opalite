#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 color;

layout(push_constant) uniform Locals {
    mat4 model;
} locals;

layout(location = 0) out vec3 v_color;

void main() {
    v_color = color;
    gl_Position = locals.model * vec4(position, 1.0);
}
