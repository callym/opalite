#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec4 v_color;
layout(location = 1) in vec2 v_uv;
layout(location = 2) flat in uint v_mode;

layout(set = 0, binding = 0) uniform texture2D tex_texture;
layout(set = 0, binding = 1) uniform sampler tex_sampler;

layout(location = 0) out vec4 Target0;

void main() {
    // text
    if (v_mode == uint(0)) {
        float alpha = texture(sampler2D(tex_texture, tex_sampler), v_uv).a;
        Target0 = v_color * vec4(1.0, 1.0, 1.0, alpha);
    // image
    } else if (v_mode == uint(1)) {
        Target0 = texture(sampler2D(tex_texture, tex_sampler), v_uv);
    // geometry
    } else {
        Target0 = v_color;
    }
}
