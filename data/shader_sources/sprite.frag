#version 450
// Uniform
layout (binding = 0) uniform sampler2D sampler_Color;
// In
layout (location = 0) in vec2 in_TexCoord;
// Out
layout (location = 0) out vec4 out_Color;
// Entry
void main() {
    out_Color = texture(sampler_Color, in_TexCoord);
}
