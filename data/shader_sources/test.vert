#version 450

layout (location = 0) out vec4 out_Color;

out gl_PerVertex
{
    vec4 gl_Position;
};

const vec3 POSITION[3] = vec3[](
	vec3(-1.0, -1.0, 0.0),
	vec3(1.0, -1.0, 0.0),
	vec3(0.0, 1.0, 0.0)
);

const vec3 COLOR[3] = vec3[](
	vec3(1.0, 0.0, 0.0),
	vec3(0.0, 1.0, 0.0),
	vec3(0.0, 0.0, 1.0)
);

void main() {
	out_Color = vec4(COLOR[gl_VertexIndex], 1.0);
	gl_Position = vec4(POSITION[gl_VertexIndex], 1.0);
}
