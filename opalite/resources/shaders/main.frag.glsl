#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 v_color;
layout(location = 1) in vec2 v_uv;

layout(push_constant) uniform Material {
    layout(offset = 64) vec4 diffuse;
} material;

layout(set = 0, binding = 1) uniform texture2D diffuse_texture;
layout(set = 0, binding = 2) uniform sampler diffuse_sampler;

layout(location = 0) out vec4 Target0;

void main() {
    Target0 = texture(sampler2D(diffuse_texture, diffuse_sampler), v_uv) * vec4(v_color, 1.0) * material.diffuse;
}
