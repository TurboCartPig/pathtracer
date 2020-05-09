#version 450

in vec2 position;

out vec2 tex_coord;

void main() {
	tex_coord = position;
	gl_Position = vec4(position, 0.0, 1.0);
}
