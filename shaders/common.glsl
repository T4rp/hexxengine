layout(set = 0, binding = 0) uniform CameraUniform {
	mat4 proj;
	mat4 view;
	vec4 cameraPos;
	mat4 lightProj;
	mat4 lightView;
} cameraUbo;

layout(set = 0, binding = 1) uniform SceneUniform {
	vec4 sunDir;
	vec4 sunCol;
	vec4 ambientCol;
} sceneUbo;


layout (set = 0, binding = 2) uniform sampler2D shadowMapText;
layout (set = 0, binding = 3) uniform samplerCube cubemapText;

layout (set = 1, binding = 0) uniform sampler2D text;

layout(set = 2, binding = 0) uniform MaterialUniform {
	vec2 uvScale;
	float shininess;
	int flags;
} materialUbo;

#define SHINE 32
#define MATERIAL_FLAG_UV 1
#define MATERIAL_FLAG_MODEL_SPACE 2

