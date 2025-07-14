#version 330 core

precision mediump float;
uniform sampler2D tex;

out vec3 FragPos;
in vec2 TexCoord;
in vec4 Color;
in vec3 Normal;
out vec4 out_color;

void main() {
    vec3 norm = normalize(Normal);
    vec3 lightDir = normalize(vec3(0.0, 1.0, 1.0) - FragPos); // Assuming a light source in the positive Z direction

    float diff = max(dot(norm, lightDir), 0.0);
    vec3 diffuse = diff * vec3(1.0, 1.0, 1.0); // White light
    vec3 ambient = vec3(0.3, 0.3, 0.3); // Ambient light

    vec3 finalColor = (ambient + diffuse) * Color.rgb * texture(tex, TexCoord).rgb;

    out_color = vec4(pow(finalColor, vec3(1.6)), 1.0);
}