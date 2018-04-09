#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec4 v_color;
layout(location = 1) in vec2 v_uv;

#define MAX_LIGHTS 8

struct Light {
    uint ty;
    vec3 color;
    vec3 position;
};

layout(set = 0, binding = 1) uniform Lights {
    uint len;
    Light lights[MAX_LIGHTS];
} lights;

layout(push_constant) uniform Material {
    layout(offset = 64) vec4 diffuse;
} material;

layout(set = 1, binding = 0) uniform texture2D diffuse_texture;
layout(set = 1, binding = 1) uniform sampler diffuse_sampler;

layout(location = 0) out vec4 Target0;

void main() {
    Target0 = texture(sampler2D(diffuse_texture, diffuse_sampler), v_uv) * v_color * material.diffuse;
}
