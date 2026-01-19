#version 450

layout (location = 0) in vec3 inColor;
layout (location = 1) in vec2 inUv;
layout (location = 2) in vec3 inNorm;
layout (location = 3) in vec3 inPos;

layout (location = 0) out vec4 outFragColor;

layout(set = 0, binding = 0) uniform SceneUniform {
	mat4 proj;
	mat4 view;
	vec4 cameraPos;
	vec4 sunDir;
	vec4 sunCol;
	vec4 ambientCol;
} sceneUbo;

layout (set = 1, binding = 0) uniform sampler2D text;

const float shine = 32.0;

void main() {
	float lightPower = sceneUbo.sunCol.w;
	vec3 lightColor = sceneUbo.sunCol.xyz;
	vec3 ambientColor = sceneUbo.ambientCol.xyz;

	vec3 norm = normalize(inNorm);
	vec3 lightDir = normalize(-sceneUbo.sunDir.xyz);
	vec3 viewDir = normalize(sceneUbo.cameraPos.xyz - inPos);
	vec3 halfDir = normalize(lightDir + viewDir);

	float diffuse = max(dot(norm, lightDir), 0.0);

	float specular = 0.0;
	if (diffuse > 0.0) {
		specular = pow(max(dot(halfDir, norm), 0.0), shine);
	}

	vec3 diffuseColor = (texture(text, inUv) * vec4(inColor, 1.0)).xyz;

	outFragColor = vec4(diffuseColor * ambientColor + diffuseColor * diffuse * lightColor * lightPower + specular * lightColor * lightPower, 1.0);
}
