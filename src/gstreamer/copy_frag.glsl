#version 300 es
precision highp float;

uniform sampler2D tex;
in vec2 tex_coord;

out vec4 f_color;

void main() {
  
  f_color = pow(texture(tex, tex_coord), vec4(2.2));
}
