layout(set = 0, binding = 0) uniform CameraUniform {
	mat4 proj;
	mat4 view;
} cameraUbo;

layout (set = 0, binding = 1) uniform sampler2D glyphAtlasText;

layout (set = 1, binding = 0) uniform sampler2D text;

