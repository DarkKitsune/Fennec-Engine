#version 450

// Vertex attributes
layout (location = 0) in vec3 in_Position;
layout (location = 1) in vec3 in_Normal;
layout (location = 2) in vec2 in_TexCoord;

// Instance attributes
layout (location = 3) in vec2 translation;
layout (location = 4) in vec2 scale;
layout (location = 5) in mat4 rotation;
layout (location = 9) in vec2 velocity;
layout (location = 10) in vec2 spriteLT;
layout (location = 11) in vec2 spriteRB;
layout (location = 12) in vec2 spriteCenter;
layout (location = 13) in vec4 colorBlend;

layout (binding = 0) uniform Camera
{
    mat4 Projection;
} camera;

layout (binding = 1) uniform Timer
{
    float time;
} timer;


layout (location = 0) out vec2 out_TexCoord;
layout (location = 1) out vec4 out_ColorBlend;

out gl_PerVertex
{
    vec4 gl_Position;
};

void main() {
	out_TexCoord = mix(spriteLT, spriteRB, in_TexCoord);
	out_ColorBlend = colorBlend;
	vec4 rotated = rotation * vec4((in_Position.xy - spriteCenter) * scale, 0.0, 1.0);
   	gl_Position = camera.Projection * (rotated + vec4(translation + velocity * timer.time, 0.0, 0.0));
}
