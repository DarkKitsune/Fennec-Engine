#version 450

layout (binding = 2) uniform sampler2D sampler_Color;

layout (location = 0) in vec2 in_TexCoord;
layout (location = 1) in vec4 in_ColorBlend;

layout (location = 0) out vec4 out_Color;

void main() {
    vec4 color = texture(sampler_Color, in_TexCoord) * in_ColorBlend;

    out_Color = color;
}
