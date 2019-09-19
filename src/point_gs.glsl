layout (points) in;
layout (triangle_strip, max_vertices = 6) out;

out vec2 g_center;
out vec2 g_point;
out float g_radius;

void main() {
  float r = 0.025; // radius
  float r2 = r * .5;
  vec2 p = gl_in[0].gl_Position.xy;

  gl_Position = vec4(p + vec2(-r2, -r2), 0., 1.);
  g_center = p;
  g_point = gl_Position.xy;
  g_radius = r2;
  EmitVertex();
  gl_Position = vec4(p + vec2(r2, -r2), 0., 1.);
  g_center = p;
  g_point = gl_Position.xy;
  g_radius = r2;
  EmitVertex();
  gl_Position = vec4(p + vec2(-r2, r2), 0., 1.);
  g_center = p;
  g_point = gl_Position.xy;
  g_radius = r2;
  EmitVertex();
  EndPrimitive();

  gl_Position = vec4(p + vec2(r2, -r2), 0., 1.);
  g_center = p;
  g_point = gl_Position.xy;
  g_radius = r2;
  EmitVertex();
  gl_Position = vec4(p + vec2(r2, r2), 0., 1.);
  g_center = p;
  g_point = gl_Position.xy;
  g_radius = r2;
  EmitVertex();
  gl_Position = vec4(p + vec2(-r2, r2), 0., 1.);
  g_center = p;
  g_point = gl_Position.xy;
  g_radius = r2;
  EmitVertex();
  EndPrimitive();
}
