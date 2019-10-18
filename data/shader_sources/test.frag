#version 450

layout (binding = 1) uniform sampler2D sampler_Color;

layout (location = 0) in vec4 in_Color;
layout (location = 1) in vec2 in_TexCoord;

layout (location = 0) out vec4 out_Color;

void main() {
    out_Color = texture(sampler_Color, in_TexCoord) * in_Color;
}
