#version 330 core

precision mediump float;
uniform sampler2D tex;

struct Light {
    vec4 color;
    vec4 position;
};

uniform Light lights[3];
uniform vec4 ambient;

in vec3 FragPos;
in vec2 TexCoord;
in vec4 Color;
in vec3 Normal;
out vec4 out_color;

const vec3 viewPos = vec3(0.0, 2.5, 10.0);

void main() {
    vec3 norm = normalize(Normal);
    vec3 viewDir = normalize(viewPos - FragPos);
    vec3 result = vec3(0.0);

    for (int i = 0; i < 3; i++) {
        // Diffuse
        vec3 lightDir = normalize(vec3(lights[i].position * 2.0 - 1.0) - FragPos);
        float diff = max(dot(norm, lightDir), 0.0);

        // Specular
        vec3 reflectDir = reflect(-lightDir, norm);
        float spec = pow(max(dot(viewDir, reflectDir), 0.0), 32.0);

        vec3 diffuse = diff * vec3(lights[i].color);
        vec3 specular = spec * vec3(lights[i].color) * 0.5;

        result += diffuse + specular;
    }

    vec3 color = (vec3(ambient) + result) * texture(tex, TexCoord).rgb;

    color = color / (color + vec3(1.0));
    color = pow(color, vec3(1.0 / 2.2));

//    vec3 finalColor = (ambient + diffuse) * Color.rgb * texture(tex, TexCoord).rgb;

    out_color = vec4(color, 1.0);
}