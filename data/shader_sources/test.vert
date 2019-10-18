#version 450

layout (location = 0) out vec4 out_Color;
layout (location = 1) out vec2 out_TexCoord;

layout (binding = 0) uniform uniform_block_Colors {
	vec4 color[3];
} uniform_Colors;

out gl_PerVertex
{
    vec4 gl_Position;
};

const vec3 POSITION[3] = vec3[](
	vec3(-1.0, -1.0, 0.0),
	vec3(1.0, -1.0, 0.0),
	vec3(0.0, 1.0, 0.0)
);

const vec2 TEX_COORD[3] = vec2[](
	vec2(0.0, 0.0),
	vec2(1.0, 0.0),
	vec2(0.5, 1.0)
);

void main() {
	out_Color = uniform_Colors.color[gl_VertexIndex];
	out_TexCoord = TEX_COORD[gl_VertexIndex];
	gl_Position = vec4(POSITION[gl_VertexIndex], 1.0);
}
