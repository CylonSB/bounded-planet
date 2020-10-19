#version 450

layout(location = 0) in vec2 v_Uv;
layout(location = 1) in vec4 v_Color;

layout(location = 0) out vec4 o_Target;

// #ifdef EGUINODE_TEXTURE
layout(set = 1, binding = 0) uniform texture2D EguiNode_texture;
layout(set = 1, binding = 1) uniform sampler EguiNode_texture_sampler;
// #endif

void main() {
    vec4 color = v_Color;
    
// #ifdef EGUINODE_TEXTURE
    color.a *= texture(
        sampler2D(EguiNode_texture, EguiNode_texture_sampler),
        v_Uv
    ).r;
// #endif

    o_Target = color;
}