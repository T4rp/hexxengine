#version 450

#include "common.glsl"

layout (location = 0) in vec3 inColor;
layout (location = 1) in vec2 inUv;
layout (location = 2) in vec3 inNorm;
layout (location = 3) in vec3 inPos;
layout (location = 4) in vec3 inScale;
layout (location = 5) in vec3 inObjNorm;
layout (location = 6) in vec4 inPosLightSpace;
layout (location = 7) in float inOpacity;

layout (location = 0) out vec4 outFragColor;

float getShadow(vec4 shadowCoord, vec2 off) {
	float shadow = 1.0;
	vec4 shadowCoordNdc = shadowCoord / shadowCoord.w;

	if (shadowCoordNdc.z > -1.0 && shadowCoordNdc.z < 1.0) {
		vec2 shadowUv = shadowCoordNdc.xy;
		shadowUv = shadowUv * 0.5 + 0.5;

		float closestDepth = texture(shadowMapText, shadowUv + off).r;
		float currentDepth = shadowCoordNdc.z;

		if (shadowCoordNdc.w > 0.0 && currentDepth > closestDepth) {
			shadow = 0.0;
		}
	}

	return shadow;
}

float shadowFilterPcf(vec4 shadowCoord) {
	ivec2 texDim = textureSize(shadowMapText, 0);
	float scale = 1.0;
	float dx = scale * 1.0 / float(texDim.x);
	float dy = scale * 1.0 / float(texDim.y);

	float shadowFactor = 0.0;
	int count = 0;
	int range = 1;

	for (int x = -range; x <= range; x++) {
		for (int y = -range; y <= range; y++) {
			shadowFactor += getShadow(shadowCoord, vec2(dx*x, dy*y));
			count++;
		}
	
	}

	return shadowFactor / count;
}

void main() {
	vec2 uv = inUv;
	int materialFlags = materialUbo.flags;

	if ((materialFlags & MATERIAL_FLAG_MODEL_SPACE) != 0) { 
		if (abs(inObjNorm.y) > 0.5) 
			uv = inUv * inScale.xz;
		else if (abs(inObjNorm.z) > 0.5)
			uv = inUv * inScale.yx;
		else
			uv = inUv * inScale.yz;
	}

	uv = uv * materialUbo.uvScale;

	float lightPower = sceneUbo.sunCol.w;
	vec3 lightColor = sceneUbo.sunCol.xyz;
	vec3 ambientColor = sceneUbo.ambientCol.xyz;

	vec3 norm = normalize(inNorm);
	vec3 lightDir = normalize(-sceneUbo.sunDir.xyz);
	vec3 viewDir = normalize(cameraUbo.cameraPos.xyz - inPos);
	vec3 halfDir = normalize(lightDir + viewDir);

	float diffuse = max(dot(norm, lightDir), 0.0);
	float specular = pow(max(dot(halfDir, norm), 0.0), materialUbo.shininess);
	float shadow = shadowFilterPcf(inPosLightSpace);
	vec3 diffuseColor = (texture(text, uv) * vec4(inColor, 1.0)).xyz;

	outFragColor = vec4(diffuseColor * ambientColor + (1.0 - shadow) * (diffuseColor * diffuse * lightColor * lightPower + specular * lightColor * lightPower), inOpacity);
}

