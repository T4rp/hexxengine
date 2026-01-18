#version 450

layout (location = 0) in vec3 inColor;
layout (location = 1) in vec2 inUv;
layout (location = 2) in vec3 inNorm;
layout (location = 3) in vec3 inPos;

layout (location = 0) out vec4 outFragColor;

layout(set = 0, binding = 0) uniform SceneUniform {
    mat4 proj;
    mat4 view;
    vec4 sunDir;
    vec4 sunCol;
    vec4 ambientCol;
} sceneUbo;

layout (set = 1, binding = 0) uniform sampler2D text;

void main() {
	vec3 lightColor = vec3(sceneUbo.sunCol);
	float lightPower = sceneUbo.sunCol.w;
	vec3 ambientColor = vec3(sceneUbo.ambientCol);
	vec3 specColor = lightColor;
	float shininess = 1.0;

	vec3 norm = normalize(inNorm);
	vec3 lightDir = vec3(sceneUbo.sunDir);
	float distance = dot(lightDir, lightDir);
	lightDir = normalize(lightDir);

	float lambertian = max(dot(lightDir, norm), 0.0);
	float specular = 0.0;

	if (lambertian > 0.0) {
		vec3 viewDir = normalize(-inPos);

		vec3 halfDir = normalize(lightDir + viewDir);
		float specAngle = max(dot(halfDir, norm), 0.0);
		specular = pow(specAngle, shininess);
	}

	vec4 diffuseColor = texture(text, inUv) * vec4(inColor, 1.0);

	vec3 colorLinear = ambientColor +
	vec3(diffuseColor) * lambertian * lightColor * lightPower / distance +
	specColor * specular * lightColor * lightPower / distance;

	outFragColor = diffuseColor * vec4(colorLinear, 1.0);
}
