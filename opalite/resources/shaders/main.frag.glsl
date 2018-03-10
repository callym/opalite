#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) out vec4 Target0;

layout(set = 0, binding = 0) uniform texture2D u_Texture;
layout(set = 0, binding = 1) uniform sampler u_Sampler;

void main() {
    Target0 = vec4(1.0, 0.0, 0.0, 1.0);
}
