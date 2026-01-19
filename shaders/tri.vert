#version 450

layout (location = 0) in vec3 inPos;
layout (location = 1) in vec3 inNorm;
layout (location = 2) in vec2 inUv;

layout(location = 3) in mat4 inModel;
layout(location = 7) in mat3 inModelNormal;
layout(location = 10) in vec3 inColor;

layout (location = 0) out vec3 outColor;
layout (location = 1) out vec2 outUv;
layout (location = 2) out vec3 outNorm;
layout (location = 3) out vec3 outPos;

layout(set = 0, binding = 0) uniform SceneUniform {
	mat4 proj;
	mat4 view;
	vec4 cameraPos;
	vec4 sunDir;
	vec4 sunCol;
	vec4 ambientColor;
} sceneUbo;

void main() {
	gl_Position = sceneUbo.proj * sceneUbo.view * inModel * vec4(inPos, 1.0f);
	outColor = inColor;
	outUv = inUv;
	outNorm = inModelNormal * inNorm;
	outPos = vec3(inModel * vec4(inPos, 1.0));
}
