layout (points) in;
layout (triangle_strip, max_vertices = 6) out;

in float v_radius[];
in vec3 v_color[];

out vec2 g_center;
out vec2 g_point;
out float g_radius;
out vec3 g_color;

void main() {
  vec2 p = gl_in[0].gl_Position.xy;
  float r = v_radius[0];
  vec3 color = v_color[0];

  gl_Position = vec4(p + vec2(-r, -r), 0., 1.);
  g_center = p;
  g_point = gl_Position.xy;
  g_radius = r;
  g_color = color;
  EmitVertex();
  gl_Position = vec4(p + vec2(r, -r), 0., 1.);
  g_center = p;
  g_point = gl_Position.xy;
  g_radius = r;
  g_color = color;
  EmitVertex();
  gl_Position = vec4(p + vec2(-r, r), 0., 1.);
  g_center = p;
  g_point = gl_Position.xy;
  g_radius = r;
  g_color = color;
  EmitVertex();
  EndPrimitive();

  gl_Position = vec4(p + vec2(r, -r), 0., 1.);
  g_center = p;
  g_point = gl_Position.xy;
  g_radius = r;
  g_color = color;
  EmitVertex();
  gl_Position = vec4(p + vec2(r, r), 0., 1.);
  g_center = p;
  g_point = gl_Position.xy;
  g_radius = r;
  g_color = color;
  EmitVertex();
  gl_Position = vec4(p + vec2(-r, r), 0., 1.);
  g_center = p;
  g_point = gl_Position.xy;
  g_radius = r;
  g_color = color;
  EmitVertex();
  EndPrimitive();
}
