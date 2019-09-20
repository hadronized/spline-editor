use luminance::blending::{Equation, Factor};
use luminance::context::GraphicsContext;
use luminance::depth_test::DepthTest;
use luminance::render_state::RenderState;
use luminance::shader::program::Program;
use luminance::tess::{Mode, Tess, TessBuilder};
use luminance_derive::{Semantics, Vertex};
use luminance_glfw::{Action, GlfwSurface, Key as GKey, MouseButton, Surface, WindowDim, WindowEvent, WindowOpt};
use splines::{Interpolation, Key, Spline};

#[derive(Copy, Clone, Debug, Semantics)]
pub enum Semantics {
  #[sem(name = "position", repr = "[f32; 2]", wrapper = "VPos")]
  Position,
  #[sem(name = "radius", repr = "f32", wrapper = "VRadius")]
  Radius,
  #[sem(name = "color", repr = "[f32; 3]", wrapper = "VColor")]
  Color,
}

#[derive(Clone, Debug, Vertex)]
#[vertex(sem = "Semantics")]
#[repr(C)]
struct LineVertex(VPos);

#[derive(Clone, Debug, Vertex)]
#[vertex(sem = "Semantics")]
#[repr(C)]
struct PointVertex(VPos, VColor, VRadius);

const DELTA_T: f32 = 0.01;
const POINT_SELECTION_DIST: f32 = DELTA_T * 8.;

/// Rebuild tessellation based on control points for lines.
fn build_lines<C>(ctx: &mut C, spline: &Spline<f32, f32>) -> Result<Tess, String> where C: GraphicsContext {
  let mut vertices = Vec::new();
  let keys = spline.keys();

  if !keys.is_empty() {
    let up_t = keys.last().unwrap().t;
    let mut t = keys[0].t;

    while t <= up_t {
      vertices.push(LineVertex::new(VPos::new([t, spline.clamped_sample(t).unwrap()])));
      t += DELTA_T;
    }
  }

  TessBuilder::new(ctx)
    .set_mode(Mode::LineStrip)
    .add_vertices(vertices)
    .build()
    .map_err(|e| format!("{:?}", e))
}

/// Rebuild tessellation based on control points for points.
fn build_points<C>(ctx: &mut C, spline: &Spline<f32, f32>, selected: Option<usize>) -> Result<Tess, String> where C: GraphicsContext {
  let mut vertices = Vec::new();
  let keys = spline.keys();

  if !keys.is_empty() {
    for cp in keys {
      vertices.push(
        PointVertex::new(
          VPos::new([cp.t, cp.value]),
          VColor::new([0.5, 0.5, 1.]),
          VRadius::new(0.025 / 2.),
        )
      );
    }
  }

  if let Some(i) = selected {
    vertices[i].1 = VColor::new([1., 0.5, 0.5]);
    vertices[i].2 = VRadius::new(0.05 / 2.);
  }

  TessBuilder::new(ctx)
    .set_mode(Mode::Point)
    .add_vertices(vertices)
    .build()
    .map_err(|e| format!("{:?}", e))
}

const LINE_VS_SRC: &str = include_str!("vs.glsl");
const LINE_FS_SRC: &str = include_str!("fs.glsl");
const POINT_VS_SRC: &str = include_str!("point_vs.glsl");
const POINT_GS_SRC: &str = include_str!("point_gs.glsl");
const POINT_FS_SRC: &str = include_str!("point_fs.glsl");

fn main() {
  let mut surface = GlfwSurface::new(WindowDim::Windowed(800, 800), "spline editor", WindowOpt::default())
    .expect("create surface");

  // the actual spline
  let mut spline: Spline<f32, f32> = Spline::from_vec(Vec::new());

  // misc
  let mut cursor_pos: Option<[f32; 2]> = None;
  let mut selected_point = None;

  // tessellated curve
  let mut tess_curve = build_lines(&mut surface, &spline).unwrap();
  let mut tess_points = build_points(&mut surface, &spline, selected_point).unwrap();

  let point_program = Program::<Semantics, (), ()>::from_strings(None, POINT_VS_SRC, POINT_GS_SRC, POINT_FS_SRC)
    .expect("shader program")
    .ignore_warnings();
  let line_program = Program::<Semantics, (), ()>::from_strings(None, LINE_VS_SRC, None, LINE_FS_SRC)
    .expect("shader program")
    .ignore_warnings();

  'app: loop {
    let [viewport_w, viewport_h] = surface.size();
    let mut rebuild_tess = false;

    // event handling
    for event in surface.poll_events() {
      match event {
        WindowEvent::Close | WindowEvent::Key(GKey::Escape, _, Action::Release, _) => break 'app,

        WindowEvent::CursorPos(x, y) => {
          cursor_pos = Some([x as f32 / viewport_w as f32, 1. - 2. * y as f32 / viewport_h as f32]);
        }

        WindowEvent::MouseButton(MouseButton::Button1, Action::Release, _) => {
          if let Some([x, y]) = cursor_pos {
            println!("adding a point ({}, {})", x, y);

            let mut cps = spline.keys().to_owned();
            cps.push(Key::new(x, y, Interpolation::Cosine));
            spline = Spline::from_vec(cps);

            selected_point = None;
            rebuild_tess = true;
          }
        }

        WindowEvent::MouseButton(MouseButton::Button2, Action::Release, _) => {
          if let Some([x, y]) = cursor_pos {
            let mut found = None;

            // we want to select a point; check if any is nearby
            for (i, p) in spline.keys().iter().enumerate() {
              let [x, y] = [x as f32, y as f32];
              let [px, py] = [p.t, p.value];
              let dist = ((x - px).powf(2.) + (y - py).powf(2.)).sqrt();

              if dist <= POINT_SELECTION_DIST {
                match found {
                  Some((_, prev_dist)) if dist < prev_dist => {
                    found = Some((i, dist));
                  }

                  None => {
                    found = Some((i, dist));
                  }

                  _ => ()
                }
              }
            }

            if let Some((i, _)) = found {
              println!("selecting point {}", i);
              selected_point = Some(i);
            } else {
              selected_point = None;
            }

            rebuild_tess = true;
          }
        }

        WindowEvent::Key(GKey::Backspace, _, Action::Release, _) => {
          if let Some(i) = selected_point {
            let mut keys: Vec<_> = spline.keys().into_iter().cloned().collect();
            keys.swap_remove(i);
            spline = Spline::from_vec(keys);

            selected_point =None;
            rebuild_tess = true;
          }
        }

        _ => ()
      }
    }

    if rebuild_tess {
      tess_curve = build_lines(&mut surface, &spline).expect("control point re-tessellation");
      tess_points = build_points(&mut surface, &spline, selected_point).expect("control point re-tessellation");
    }

    // render
    let back_buffer = surface.back_buffer().unwrap();
    let render_state = RenderState::default()
      .set_blending(Some((Equation::Additive, Factor::SrcAlpha, Factor::SrcAlphaComplement)))
      .set_depth_test(DepthTest::Off);

    surface.pipeline_builder().pipeline(&back_buffer, [0., 0., 0., 0.], |_, mut shd_gate| {
      // lines
      shd_gate.shade(&line_program, |_, mut rdr_gate| {
        rdr_gate.render(render_state, |mut tess_gate| {
            tess_gate.render(&tess_curve);
        });
      });

      // points
      shd_gate.shade(&point_program, |_, mut rdr_gate| {
        rdr_gate.render(render_state, |mut tess_gate| {
            tess_gate.render(&tess_points);
        });
      });
    });

    surface.swap_buffers();
  }
}
