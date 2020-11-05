#version 450

layout(location = 0) in vec3 BevyEguiVertex_Position;
layout(location = 1) in vec3 BevyEguiVertex_Normal;
layout(location = 2) in vec2 BevyEguiVertex_Uv;
layout(location = 3) in vec4 BevyEguiVertex_Color;
layout(location = 4) in vec2 BevyEguiVertex_ClipMin;
layout(location = 5) in vec2 BevyEguiVertex_ClipMax;

layout(location = 0) out vec2 v_Uv;
layout(location = 1) out vec4 v_Color;

out gl_PerVertex
{
    vec4 gl_Position;
    float gl_ClipDistance[4];
};

layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};

layout(set = 2, binding = 0) uniform Transform {
    mat4 Object;
};

// 0-1 linear  from  0-255 sRGB
vec3 linear_from_srgb(vec3 srgb) {
    bvec3 cutoff = lessThan(srgb, vec3(10.31475));
    vec3 lower = srgb / vec3(3294.6);
    vec3 higher = pow((srgb + vec3(14.025)) / vec3(269.025), vec3(2.4));
    return mix(higher, lower, cutoff);
}
vec4 linear_from_srgba(vec4 srgba) {
    return vec4(linear_from_srgb(srgba.rgb), srgba.a / 255.0);
}

void main() {
    vec2 pos = vec2(BevyEguiVertex_Position.x, BevyEguiVertex_Position.y);

    vec2 clip01 = pos - BevyEguiVertex_ClipMin;
    vec2 clip23 = BevyEguiVertex_ClipMax - pos;

    gl_ClipDistance[0] = clip01.x;
    gl_ClipDistance[1] = clip01.y;
    gl_ClipDistance[2] = clip23.x;
    gl_ClipDistance[3] = clip23.y;
    
    v_Uv = BevyEguiVertex_Uv;
    v_Color = linear_from_srgba(BevyEguiVertex_Color);

    // Correct origin due to Bevy issues
    mat4 modified_proj = ViewProj;
    modified_proj[1][1] *= -1;
    modified_proj[3][1] *= -1;

    gl_Position = modified_proj * Object * vec4(BevyEguiVertex_Position, 1.0);
}