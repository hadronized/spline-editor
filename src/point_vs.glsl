in vec2 position;
in float radius;
in vec3 color;

out float v_radius;
out vec3 v_color;

void main() {
  v_radius = radius;
  v_color = color;
  gl_Position = vec4(position.x * 2. - 1., position.y, 0., 1.);
}
