#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(constant_id = 0) const uint MAX_LIGHTS = 8;

struct Light {
    vec3 color;
    vec3 position;
    uint ty;
};

layout(location = 0) in vec3 v_position;
layout(location = 1) in vec4 v_color;
layout(location = 2) in vec2 v_uv;
layout(location = 3) in vec3 v_normal;

layout(set = 0, binding = 0) uniform Locals {
    mat4 proj_view;
    vec3 camera_position;
};

layout(set = 0, binding = 1) uniform Lights {
    uint len;
    Light lights[MAX_LIGHTS];
};

layout(push_constant) uniform Material {
    layout(offset = 128) vec4 diffuse;
    float specular;
} material;

layout(set = 1, binding = 0) uniform texture2D diffuse_texture;
layout(set = 1, binding = 1) uniform sampler diffuse_sampler;

layout(location = 0) out vec4 Target0;

vec3 point_light(Light light, vec3 normal) {
    vec3 light_direction = normalize(light.position - v_position);
    vec3 view_direction = normalize(camera_position - v_position);
    vec3 halfway_direction = normalize(light_direction + view_direction);

    float ambient_factor = 0.1;
    vec3 ambient = ambient_factor * light.color;

    float diffuse_factor = max(dot(normal, light_direction), 0.0);
    vec3 diffuse = diffuse_factor * light.color;

    float specular_factor = pow(max(dot(normal, halfway_direction), 0.0), material.specular);
    vec3 specular = specular_factor * light.color;

    return ambient + diffuse + specular;
}

void main() {
    vec3 normal = normalize(v_normal);

    vec4 diffuse = v_color * material.diffuse * texture(sampler2D(diffuse_texture, diffuse_sampler), v_uv);

    vec3 color = vec3(0.0);
    for (uint i = 0; i <= len; i++) {
        Light light = lights[i];

        if (light.ty == 0) {
            continue;
        }

        if (light.ty == 1) {
            color += point_light(light, normal);
        }
    }

    Target0 = diffuse * vec4(color, 1.0);
}
