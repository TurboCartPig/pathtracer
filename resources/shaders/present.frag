#version 450

in vec2 tex_coord;

out vec4 color;

uniform sampler2D tex;

void main() {
	/* color = vec4(1.0, 0.0, 0.0, 1.0); */
	gl_FragColor = vec4(texture(tex, tex_coord), 1.0);
}
