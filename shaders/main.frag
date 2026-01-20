#version 450

#include "common.glsl"

layout (location = 0) in vec3 inColor;
layout (location = 1) in vec2 inUv;
layout (location = 2) in vec3 inNorm;
layout (location = 3) in vec3 inPos;
layout (location = 4) in vec3 inScale;
layout (location = 5) in vec3 inObjNorm;

layout (location = 0) out vec4 outFragColor;

void main() {
	vec2 uv;

	if (abs(inObjNorm.y) > 0.5) 
		uv = inUv * inScale.xz;
	else if (abs(inObjNorm.z) > 0.5)
		uv = inUv * inScale.yx;
	else
		uv = inUv * inScale.yz;

	float lightPower = sceneUbo.sunCol.w;
	vec3 lightColor = sceneUbo.sunCol.xyz;
	vec3 ambientColor = sceneUbo.ambientCol.xyz;

	vec3 norm = normalize(inNorm);
	vec3 lightDir = normalize(-sceneUbo.sunDir.xyz);
	vec3 viewDir = normalize(sceneUbo.cameraPos.xyz - inPos);
	vec3 halfDir = normalize(lightDir + viewDir);

	float diffuse = max(dot(norm, lightDir), 0.0);

	float specular = pow(max(dot(halfDir, norm), 0.0), SHINE);

	vec3 diffuseColor = (texture(text, uv) * vec4(inColor, 1.0)).xyz;

	outFragColor = vec4(diffuseColor * ambientColor + diffuseColor * diffuse * lightColor * lightPower + specular * lightColor * lightPower, 1.0);
}

