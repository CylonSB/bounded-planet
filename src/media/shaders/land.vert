#version 450

layout(location = 0) in vec3 VertexPosition;
layout(set = 0, binding = 0) in uniform Camera {
    mat4 ViewProj;
};
layout(set = 1, binding = 0) in uniform Transform {
    mat4 Model;
};

layout(set = 2, binding = 0) in uniform sampler2D HeightMap;

layout(location = 1) out varying float v_height;

void main() {
    float HeightRange = 10;
    vec2 vertexCoordinates = vec2(VertexPosition.x, VertexPosition.z);
    float height = texture2D(HeightMap, vertexCoordinates).r;
    float h = height * HeightRange;    
    vec3 position = VertexPosition + vec3(0, h, 0);

    gl_Position = ViewProj * position;
    v_height = height;

}






