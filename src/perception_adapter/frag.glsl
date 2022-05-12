#version 300 es
precision highp float;

in vec2 tex_coord;
out vec4 color;
uniform sampler2D in_texture;
uniform float scale_factor;

void main() {
  vec4 c = texture(in_texture, tex_coord);
  c = 2.0*c - 1.0;
  c = c*scale_factor;
  c = (c + 1.0)/2.0;

  color = c;
}
