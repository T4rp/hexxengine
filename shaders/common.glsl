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

layout (set = 1, binding = 0) uniform sampler2D text;

#define SHINE 32

