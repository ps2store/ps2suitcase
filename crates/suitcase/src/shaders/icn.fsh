#version 330 core

precision mediump float;
uniform sampler2D tex;
in vec2 TexCoord;
out vec4 out_color;
void main() {
    out_color = texture(tex, TexCoord);
}