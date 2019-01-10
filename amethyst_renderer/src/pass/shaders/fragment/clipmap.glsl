#version 150 core

layout (std140) uniform FragmentArgs {
    uint point_light_count;
    uint directional_light_count;
};

struct PointLight {
    vec3 position;
    vec3 color;
    float pad; // Workaround for bug in mac's implementation of opengl (loads garbage when accessing members of structures in arrays with dynamic indices).
    float intensity;
};

layout (std140) uniform PointLights {
    PointLight plight[128];
};

struct DirectionalLight {
    vec3 color;
    vec3 direction;
};

layout (std140) uniform DirectionalLights {
    DirectionalLight dlight[16];
};

uniform sampler2D normal;
uniform sampler1D zBasedColor;

in VertexData {
    vec3 position;
    vec3 normal;
    vec3 tangent;
    vec2 tex_coord;
    vec4 color;
    vec2 uv; // coordinates for normal-map lookup
    float z; // coordinates for elevation-map lookup
    float alpha; // transition blend
} vertex;


out vec4 out_color;


void main() {
    vec3 lighting = vec3(0.0);

    vec4 normal_fc = texture(normal, uv);

    // normal_fc.xy contains normal at current (fine) level 
    // normal_fc.zw contains normal at coarser level
    // blend normals using alpha computed in vertex shader
    vec3 normal = float3((1 - alpha) * normal_fc.xy + alpha * (normal_fc.zw), 1);

    // unpack coordinates from [0, 1] to [-1, +1] range, and renormalize
    normal = normalize(normal * 2 - 1);


    // From shaded
    for (uint i = 0u; i < point_light_count; i++) {
        // Calculate diffuse light
        vec3 light_dir = normalize(plight[i].position - vertex.position);
        float diff = max(dot(light_dir, normal), 0.0);
        vec3 diffuse = diff * normalize(plight[i].color);
        // Calculate attenuation
        vec3 dist = plight[i].position - vertex.position;
        float dist2 = dot(dist, dist);
        float attenuation = (plight[i].intensity / dist2);
        lighting += diffuse * attenuation;
    }
    for (uint i = 0u; i < directional_light_count; i++) {
        vec3 dir = dlight[i].direction;
        float diff = max(dot(-dir, normal), 0.0);
        vec3 diffuse = diff * dlight[i].color;
        lighting += diffuse;
    }
    ambient_color = vec3(0.5, 0.5, 0.5);

    lighting += ambient_color;


    // assign terrain color based on its elevation 
    out_color = vec4(lighting, 1.0) * texture(zBasedColor, z);
}