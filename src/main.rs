// This is currently a prototype. The current code is pretty naive, especially in terms of
// allocation of keys in splines. Some  work must be done to clean all that stuff.

use cgmath::Vector2;
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
fn build_lines<C>(ctx: &mut C, spline: &Spline<f32, Vector2<f32>>) -> Result<Tess, String> where C: GraphicsContext {
  let mut vertices = Vec::new();
  let keys = spline.keys();

  if !keys.is_empty() {
    let up_t = keys.last().unwrap().t;
    let mut t = keys[0].t;

    while t <= up_t {
      let p = spline.clamped_sample(t).unwrap();
      vertices.push(LineVertex::new(VPos::new(p.into())));
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
fn build_points<C>(ctx: &mut C, spline: &Spline<f32, Vector2<f32>>, selected: Option<Selection>) -> Result<Tess, String> where C: GraphicsContext {
  let mut vertices = Vec::new();
  let keys = spline.keys();

  let mut specials = Vec::new();

  if !keys.is_empty() {
    for (i, cp) in keys.iter().enumerate() {
      let mut vertex = PointVertex::new(
        VPos::new(cp.value.into()),
        VColor::new([0.5, 0.5, 1.]),
        VRadius::new(0.025 / 2.),
      );

      if let Some(Selection::Point(i_sel)) = selected {
        if i_sel == i {
          vertex.1 = VColor::new([1., 0.5, 0.5]);
          vertex.2 = VRadius::new(0.025 / 2.);
        }
      }

      vertices.push(vertex);

      if let Interpolation::Bezier(u) = cp.interpolation {
        let mut vertex = PointVertex::new(
          VPos::new(u.into()),
          VColor::new([0.5, 1., 0.5]),
          VRadius::new(0.015 / 2.)
        );

        if let Some(Selection::Handle(i_sel)) = selected {
          if i_sel == i {
            vertex.1 = VColor::new([1., 0.5, 0.5]);
            vertex.2 = VRadius::new(0.015 / 2.);
          }
        }

        specials.push(vertex);
      }
    }
  }

  vertices.extend(specials);

  TessBuilder::new(ctx)
    .set_mode(Mode::Point)
    .add_vertices(vertices)
    .build()
    .map_err(|e| format!("{:?}", e))
}

#[derive(Clone, Copy, Debug)]
enum Selection {
  Point(usize),
  Handle(usize)
}

const LINE_VS_SRC: &str = include_str!("vs.glsl");
const LINE_FS_SRC: &str = include_str!("fs.glsl");
const POINT_VS_SRC: &str = include_str!("point_vs.glsl");
const POINT_GS_SRC: &str = include_str!("point_gs.glsl");
const POINT_FS_SRC: &str = include_str!("point_fs.glsl");

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 800;

fn main() {
  let mut surface = GlfwSurface::new(WindowDim::Windowed(WINDOW_WIDTH, WINDOW_HEIGHT), "spline editor", WindowOpt::default())
    .expect("create surface");

  // the actual spline
  let mut spline = Spline::from_vec(Vec::new());

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

  let mut mouse_left_pressed = false;

  'app: loop {
    let mut rebuild_tess = false;

    // event handling
    for event in surface.poll_events() {
      match event {
        WindowEvent::Close | WindowEvent::Key(GKey::Escape, _, Action::Release, _) => break 'app,

        WindowEvent::FramebufferSize(w, h) => {
          println!("new framebuffer dimensions: {}×{}", w, h);
        }

        WindowEvent::CursorPos(x, y) => {
          let xy = [x as f32 / WINDOW_WIDTH as f32, 1. - 2. * y as f32 / WINDOW_HEIGHT as f32];
          cursor_pos = Some(xy);
          println!("cursor pos: ({}, {}) -> {:?}", x, y, cursor_pos);

          if mouse_left_pressed {
            if let Some(selection) = selected_point {
              match selection {
                Selection::Point(i) => {
                  println!("moving point {} to ({}, {})", i, x, y);
                  let key = spline.get_mut(i).unwrap();
                  *key.value = Vector2::new(xy[0], xy[1]);
                }

                Selection::Handle(i) => {
                  println!("moving tangent handle of point {} to ({}, {})", i, x, y);
                  let key = spline.get_mut(i).unwrap();

                  if let Interpolation::Bezier(ref mut handle) = *key.interpolation {
                    *handle = Vector2::new(xy[0], xy[1]);
                  }
                }
              }

              rebuild_tess = true;
            }
          }
        }

        WindowEvent::MouseButton(MouseButton::Button1, Action::Release, _) => {
          mouse_left_pressed = false;

          if selected_point.is_none() {
            if let Some([x, y]) = cursor_pos {
              println!("adding a point ({}, {})", x, y);

              spline.add(Key::new(spline.len() as f32, Vector2::new(x, y), Interpolation::Cosine));

              selected_point = None;
              rebuild_tess = true;
            }
          }
        }

        WindowEvent::MouseButton(MouseButton::Button1, Action::Press, _) => {
          mouse_left_pressed = true;
        }

        WindowEvent::MouseButton(MouseButton::Button2, Action::Release, _) => {
          if let Some([x, y]) = cursor_pos {
            let [x, y] = [x as f32, y as f32];
            let mut found = None;

            // we want to select a point; check if any is nearby
            for (i, p) in spline.keys().iter().enumerate() {
              let [px, py]: [f32; 2] = p.value.into();
              let dist = ((x - px).powf(2.) + (y - py).powf(2.)).sqrt();

              if dist <= POINT_SELECTION_DIST {
                match found {
                  Some((_, prev_dist)) if dist < prev_dist => {
                    found = Some((Selection::Point(i), dist));
                  }

                  None => {
                    found = Some((Selection::Point(i), dist));
                  }

                  _ => ()
                }
              } else if let Interpolation::Bezier(ref handle) = p.interpolation {
                let [px, py]: [f32; 2] = (*handle).into();
                let dist = ((x - px).powf(2.) + (y - py).powf(2.)).sqrt();

                if dist <= POINT_SELECTION_DIST {
                  match found {
                    Some((_, prev_dist)) if dist < prev_dist => {
                      found = Some((Selection::Handle(i), dist));
                    }

                    None => {
                      found = Some((Selection::Handle(i), dist));
                    }

                    _ => ()
                  }
                }
              }
            }

            match found {
              Some((Selection::Point(i), _)) => {
                println!("selecting point {}", i);
                selected_point = Some(Selection::Point(i));
              }

              Some((Selection::Handle(i), _)) => {
                println!("selecting handle {}", i);
                selected_point = Some(Selection::Handle(i));
              }

              _ => {
                selected_point = None;
              }
            }

            rebuild_tess = true;
          }
        }

        WindowEvent::Key(GKey::Backspace, _, Action::Release, _) => {
          if let Some(Selection::Point(i)) = selected_point {
            let mut keys: Vec<_> = spline.keys().into_iter().cloned().collect();
            keys.swap_remove(i);
            spline = Spline::from_vec(keys);

            selected_point = None;
            rebuild_tess = true;
          }
        }

        WindowEvent::Key(GKey::Space, _, Action::Release, _) => {
          if let Some(Selection::Point(i)) = selected_point {
            if let Some(key) = spline.get_mut(i) {
              let prev = *key.interpolation;
              *key.interpolation = toggle_interpolation(*key.value, prev);

              println!("toggling interpolation for key {}; {:?} -> {:?}", i, prev, key.interpolation);

              rebuild_tess = true;
            }
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

fn toggle_interpolation(p: Vector2<f32>, i: Interpolation<f32, Vector2<f32>>) -> Interpolation<f32, Vector2<f32>> {
  match i {
    Interpolation::Step(_) => Interpolation::Linear,
    Interpolation::Linear => Interpolation::Cosine,
    Interpolation::Cosine => Interpolation::Bezier(p + Vector2::new(0.1, 0.1)),
    Interpolation::Bezier(_) => Interpolation::Step(0.5),
    _ => i
  }
}
