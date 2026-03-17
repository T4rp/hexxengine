#version 450

#include "common2d.glsl"

layout (location = 0) in vec2 inUv;
layout (location = 1) in vec4 inColor;

layout (location = 0) out vec4 outFragColor;

void main() {
	float col = textureLod(glyphAtlasText, inUv, 0).r;
	outFragColor = vec4(col, col, col, col) * inColor;
}
