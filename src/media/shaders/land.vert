#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};
layout(set = 1, binding = 0) uniform Transform {
    mat4 Model;
};

layout(location = 6) out float origin_distance;
layout(location = 5) out float v_height;

void main() {
    vec4 Original_Position = Model * vec4(Vertex_Position, 1.0);

    origin_distance = sqrt(pow(Original_Position.x, 2) + pow(Original_Position.z, 2));  
    float h = sin(Original_Position.x) + sin(Original_Position.z);
    vec4 position = Original_Position + vec4(0, h, 0, 0);

    gl_Position = ViewProj * position;
    v_height = h;

}






