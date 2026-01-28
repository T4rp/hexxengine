#version 450

#include "common.glsl"

layout (location = 0) in vec3 inPos;

layout (location = 0) out vec4 outFragColor;

void main() {
	vec3 direction = normalize(inPos - cameraUbo.cameraPos.xyz);
	outFragColor = vec4(texture(cubemapText, vec3(direction)).xyz, 1.0);
}

