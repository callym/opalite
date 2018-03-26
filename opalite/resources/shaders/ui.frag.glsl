#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec4 v_color;

layout(location = 0) out vec4 Target0;

void main() {
    Target0 = v_color;
}
