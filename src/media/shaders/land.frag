#version 450

layout(location = 5) in float v_height;
layout(location = 6) in float origin_distance;

layout(location = 0) out vec4 o_Target;

void main() {
    o_Target = vec4(origin_distance / 20, 0.0, v_height / 6 + 1, 0.6);
    
}