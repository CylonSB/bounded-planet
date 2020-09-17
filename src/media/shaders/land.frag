#version 450
layout(set = 2, binding = 0) uniform Sampler2d HeightMap;

layout(location = 1) in varying float v_height;

layout(location = 0) out vec4 o_Target;

void main() {
    if (v_height > 8) {
        o_Target = vec4(1.0, 1.0, 1.0, 1.0);
    } else {
        if (v_height > 1) {
            o_Target = vec4(0.2, 0.8, 0.2, 1.0);
        } else {
            o_Target = vec4(0.0, 0.1, 0.8, 1.0);
        }
    }
}