in vec2 position;

void main() {
  gl_Position = vec4(position.x * 2. - 1., position.y, 0., 1.);
}
