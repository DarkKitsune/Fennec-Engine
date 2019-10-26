#version 450
// Constants
const vec2 POSITION[4] = vec2[](
	vec2(1.0, 0.0),
	vec2(1.0, 1.0),
	vec2(0.0, 0.0),
	vec2(0.0, 1.0)
);
const vec2 TEX_COORD[4] = vec2[](
	vec2(1.0, 0.0),
	vec2(1.0, 1.0),
	vec2(0.0, 0.0),
	vec2(0.0, 1.0)
);
// In
layout (location = 0) in vec2 instance_Position;
layout (location = 1) in ivec4 instance_TileRegion;
// Out
layout (location = 0) out vec2 out_TexCoord;
// Vertex out
out gl_PerVertex
{
    vec4 gl_Position;
};
// Entry
void main() {
	out_TexCoord = TEX_COORD[gl_VertexIndex];
   	gl_Position = vec4(0.0, 0.0, 0.0, 1.0) + vec4(POSITION[gl_VertexIndex], 0.0, 0.0);
}
