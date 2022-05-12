#version 300 es
#define M_PI 3.1415926535897932384626433832795
precision highp float;

in vec2 tex_coord;
out vec4 color;
uniform float frequency;

void main() {
  float x = 2.0 * M_PI * frequency;
  float r = (sin(x*tex_coord.x) + 1.0)/2.0;
  r = pow(r,2.2);
  color = vec4(r,r,r,1.0);
}
