#version 450

layout (location = 0) in vec3 inColor;
layout (location = 1) in vec2 inUv;

layout (location = 0) out vec4 outFragColor;

layout (set = 1, binding = 0) uniform sampler2D text;

void main() 
{
	outFragColor = texture(text, inUv);
}
