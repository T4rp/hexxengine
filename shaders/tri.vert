#version 450

layout (location = 0) in vec2 inPos;
layout (location = 1) in vec2 inUv;
layout (location = 2) in vec3 inColor;

layout (location = 0) out vec3 outColor;
layout (location = 1) out vec2 outUv;

layout(binding = 0) uniform CameraUniform {
    mat4 view;
    mat4 proj;
} cameraUbo;

void main() {
	gl_Position = cameraUbo.view * cameraUbo.proj * vec4(inPos, 0.0f, 1.0f);
	outColor = inColor;
	outUv = inUv;
}
