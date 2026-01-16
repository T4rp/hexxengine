#version 450

layout (location = 0) in vec3 inPos;
layout (location = 1) in vec3 inNorm;
layout (location = 2) in vec2 inUv;

layout(location = 3) in mat4 inModel;
layout(location = 7) in vec3 inColor;

layout (location = 0) out vec3 outColor;
layout (location = 1) out vec2 outUv;

layout(binding = 0) uniform CameraUniform {
    mat4 proj;
    mat4 view;
} cameraUbo;

void main() {
	gl_Position = cameraUbo.proj * cameraUbo.view * inModel * vec4(inPos, 1.0f);
	outColor = inColor;
	outUv = inUv;
}
