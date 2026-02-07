#version 450

#include "common.glsl"

layout (location = 0) in vec3 inPos;
layout (location = 1) in vec3 inNorm;
layout (location = 2) in vec2 inUv;

layout (location = 0) out vec3 outPos;

mat4 transform = mat4(
    1000.0, 0.0,  0.0,  0.0,
    0.0, 1000.0,  0.0,  0.0,
    0.0,  0.0, 1000.0,  0.0,
    0.0,  0.0,  0.0,  1.0
);

void main() {
    mat4 view = mat4(mat3(cameraUbo.view));
	gl_Position = cameraUbo.proj * view * transform * vec4(inPos, 1.0f);
	outPos = inPos;
}
