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

const vec3 positions[3] = vec3[3](
	vec3(1.f,1.f, 0.0f),
	vec3(-1.f,1.f, 0.0f),
	vec3(0.f,-1.f, 0.0f)
);

//const array of colors for the triangle
const vec3 colors[3] = vec3[3](
	vec3(1.0f, 0.0f, 0.0f), //red
	vec3(0.0f, 1.0f, 0.0f), //green
	vec3(00.f, 0.0f, 1.0f)  //blue
);

void main() {
	//output the position of each vertex
	gl_Position = cameraUbo.view * cameraUbo.proj * vec4(inPos, 0.0f, 1.0f);
	outColor = inColor;
	outUv = inUv;
}
