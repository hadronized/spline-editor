in vec2 g_center;
in vec2 g_point;
in float g_radius;
in vec3 g_color;

out vec4 frag;

void main() {
  frag = vec4(g_color, 1. - step(g_radius, distance(g_center, g_point)));
}
