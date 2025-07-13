#version 330 core

uniform vec3 color;
precision mediump float;
out vec4 out_color;

void main() {
    out_color = vec4(color, 1);
}