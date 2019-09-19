in vec2 g_center;
in vec2 g_point;
in float g_radius;

out vec4 frag;

void main() {
  frag = vec4(.5, .5, 1., 1. - step(g_radius, distance(g_center, g_point)));
}
