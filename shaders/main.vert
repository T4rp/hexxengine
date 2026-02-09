#version 450

#include "common.glsl"

layout (location = 0) in vec3 inPos;
layout (location = 1) in vec3 inNorm;
layout (location = 2) in vec2 inUv;

layout (location = 3) in mat4 inModel;
layout (location = 7) in mat3 inModelNormal;
layout (location = 10) in vec3 inColor;
layout (location = 11) in float inOpacity;

layout (location = 0) out vec3 outColor;
layout (location = 1) out vec2 outUv;
layout (location = 2) out vec3 outNorm;
layout (location = 3) out vec3 outPos;
layout (location = 4) out vec3 outScale;
layout (location = 5) out vec3 outObjNorm;
layout (location = 6) out vec4 outPosLightSpace;
layout (location = 7) out float outOpacity;

void main() {
	vec3 scale;
	scale.x = length(inModel[0].xyz);
	scale.y = length(inModel[1].xyz);
	scale.z = length(inModel[2].xyz);

	gl_Position = cameraUbo.proj * cameraUbo.view * inModel * vec4(inPos, 1.0f);
	outColor = inColor;
	outUv = inUv;
	outNorm = inModelNormal * inNorm;
	outPos = vec3(inModel * vec4(inPos, 1.0));
	outScale = scale;
	outObjNorm = inNorm;
	outPosLightSpace = cameraUbo.lightProj * cameraUbo.lightView * inModel * vec4(inPos, 1.0f);
	outOpacity = inOpacity;
}

