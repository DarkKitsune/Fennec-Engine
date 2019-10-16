#version 450

layout (location = 0) out vec4 out_Color;

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

void main() {
	out_Color = uniform_Colors.color[gl_VertexIndex];
	gl_Position = vec4(POSITION[gl_VertexIndex], 1.0);
}
