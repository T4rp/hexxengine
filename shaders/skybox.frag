#version 450

#include "common.glsl"

layout (location = 0) in vec3 inPos;

layout (location = 0) out vec4 outFragColor;

void main() {
	outFragColor = vec4(texture(cubemapText, inPos));
}

