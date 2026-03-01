#version 450

#include "common.glsl"

layout (location = 0) in vec2 inPos;
layout (location = 1) in vec2 inUv;
layout (location = 2) in vec4 inColor;

layout (location = 0) out vec2 outUv;
layout (location = 1) out vec4 outColor;

void main() {
	gl_Position = cameraUbo.proj * cameraUbo.view * vec4(inPos, 0.0, 1.0f);
	outUv = inUv;
	outColor = inColor;
}

