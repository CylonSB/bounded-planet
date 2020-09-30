#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec2 Texture_Coordinates;
layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};
layout(set = 1, binding = 0) uniform Transform {
    mat4 Model;
};
layout(set = 2, binding = 0) uniform sampler HeightMap;

layout(location = 5) out float v_height;
layout(location = 6) out float origin_distance;

void main() {
    vec4 Original_Position = Model * vec4(Vertex_Position, 1.0);
    origin_distance = sqrt(pow(Original_Position.x, 2) + pow(Original_Position.z, 2));  
    vec4 position = Original_Position;

    gl_Position = ViewProj * position;
    v_height = position.y;

}





