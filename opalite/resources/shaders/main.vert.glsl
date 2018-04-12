#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 position;
layout(location = 1) in vec4 color;
layout(location = 2) in vec2 uv;
layout(location = 3) in vec3 normal;

layout(set = 0, binding = 0) uniform Locals {
    mat4 proj_view;
    vec3 camera_position;
};

layout(push_constant) uniform ModelLocals {
    mat4 model;
    mat4 normal;
} model;

layout(location = 0) out vec3 v_position;
layout(location = 1) out vec4 v_color;
layout(location = 2) out vec2 v_uv;
layout(location = 3) out vec3 v_normal;

void main() {
    v_position = vec3(model.model * vec4(position, 1.0));
    v_color = color;
    v_uv = uv;
    v_normal = vec3(model.normal * vec4(normal, 0.0));
    gl_Position = proj_view * model.model * vec4(position, 1.0);
}
