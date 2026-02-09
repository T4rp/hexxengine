#version 450

#include "common.glsl"

layout (location = 0) in vec3 inPos;
layout (location = 1) in vec3 inNorm;
layout (location = 2) in vec2 inUv;

layout(location = 3) in mat4 inModel;
layout(location = 7) in mat3 inModelNormal;
layout(location = 10) in vec3 inColor;
layout(location = 11) in vec3 inOpacity;

void main() {
	gl_Position = cameraUbo.lightProj * cameraUbo.lightView * inModel * vec4(inPos, 1.0f);
}

