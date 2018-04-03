#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 color;
layout(location = 2) in vec2 uv;

layout(set = 0, binding = 0) uniform Locals {
    mat4 proj_view;
};

layout(push_constant) uniform ModelLocals {
    mat4 model;
} model;

layout(location = 0) out vec3 v_color;
layout(location = 1) out vec2 v_uv;

void main() {
    v_color = color;
    v_uv = uv;
    gl_Position = proj_view * model.model * vec4(position, 1.0);
}
