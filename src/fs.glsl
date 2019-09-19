in vec2 g_center;
in vec2 g_point;
in float g_radius;

out vec3 frag;

void main() {
  frag = vec3(.5, .5, 1.) * (1. - distance(g_center, g_point) / g_radius);
}
