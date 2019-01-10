#version 150 core


layout (std140) uniform VertexArgs {
    uniform mat4 proj;
    uniform mat4 view;
    uniform mat4 model;
    uniform vec4 rgba; // unused
};

uniform vec3 camera_position;
uniform vec3 alpha_offset;
uniform vec3 one_over_width;
uniform vec4 fine_block_orig;
uniform vec4 scale_factor;
uniform float z_scale_factor;
uniform float z_tex_scale_factor;
uniform int size;

uniform sampler2DArray elevation_sampler;

in ivec2 position;

out VertexData {
    vec3 position;
    vec3 normal;
    vec3 tangent;
    vec2 tex_coord;
    vec4 color;
    vec2 uv; // coordinates for normal-map lookup
    float z; // coordinates for elevation-map lookup
    float alpha; // transition blend
} vertex;

// Vertex shader for rendering the geometry clipmap
void main() {
    vec2 grid_pos = position; //(fmod(gl_VertexID, size), floor(gl_VertexID/size)) 
    // convert from grid xy to world xy coordinates
    // Scale_factor.xy: grid spacing of current level
    // Scale_factor.zw: origin of current block within world 
    vec2 world_pos = grid_pos * scale_factor.xy + scale_factor.zw;
    // compute coordinates for vertex texture
    // Fine_block_orig.xy: 1/(w, h) of texture
    // Fine_block_orig.zw: origin of block in texture
    vec2 uv = grid_pos * fine_block_orig.xy + fine_block_orig.zw;

    // sample the vertex texture
    float zf_zd = textureLod(elevation_sampler, vec4(uv, 0), 1);
    // unpack to obtain zf and zd = (zc - zf)
    // zf is elevation value in current (fine) level
    // zc is elevation value in coarser level
    float zf = floor(zf_zd);
    float zd = frac(zf_zd) * 512 - 256; // (zd = zc - zf)

    // compute alpha (transition parameter) and blend elevation
    float2 alpha = clamp((abs(world_pos - camera_position) – alpha_offset) * one_over_width, 0, 1);
    alpha.x = max(alpha.x, alpha.y); 
    float z = zf + alpha.x * zd;
    z = z * z_scale_factor;

    vec4 vertex_position = model * vec4(world_pos, z, 1.0);
    vertex.position = vertex_position.xyz;
    vertex.uv = uv;
    vertex.z = z * z_tex_scale_factor; 
    vertex.alpha = alpha.x;
    gl_Position = proj * view * vertex_position;
}