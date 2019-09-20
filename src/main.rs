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
}

#[derive(Clone, Debug, Vertex)]
#[vertex(sem = "Semantics")]
#[repr(C)]
struct Vertex(VPos);

const DELTA_T: f32 = 0.01;

/// Rebuild tessellation based on control points.
fn build_tess<C>(ctx: &mut C, spline: &Spline<f32, f32>, mode: Mode) -> Result<Tess, String> where C: GraphicsContext {
  let mut vertices = Vec::new();

  (|| {
    let keys = spline.keys();

    if keys.is_empty() {
      return;
    }

    if let Mode::Point = mode {
      for cp in keys {
        vertices.push(Vertex::new(VPos::new([cp.t, cp.value])));
      }
    } else {
      let up_t = keys.last().unwrap().t;
      let mut t = keys[0].t;

      while t <= up_t {
        vertices.push(Vertex::new(VPos::new([t, spline.clamped_sample(t).unwrap()])));
        t += DELTA_T;
      }
    }
  })();

  TessBuilder::new(ctx)
    .set_mode(mode)
    .add_vertices(vertices)
    .build()
    .map_err(|e| format!("{:?}", e))
}

const VS_SRC: &str = include_str!("vs.glsl");
const LINE_FS_SRC: &str = include_str!("fs.glsl");
const POINT_GS_SRC: &str = include_str!("point_gs.glsl");
const POINT_FS_SRC: &str = include_str!("point_fs.glsl");

fn main() {
  let mut surface = GlfwSurface::new(WindowDim::Windowed(800, 800), "spline editor", WindowOpt::default())
    .expect("create surface");

  // the actual spline
  let mut spline: Spline<f32, f32> = Spline::from_vec(Vec::new());
  let mut cursor_pos: Option<[f32; 2]> = None;

  // tessellated curve
  let mut tess_curve = build_tess(&mut surface, &spline, Mode::LineStrip).unwrap();
  let mut tess_points = build_tess(&mut surface, &spline, Mode::Point).unwrap();

  let point_program = Program::<Semantics, (), ()>::from_strings(None, VS_SRC, POINT_GS_SRC, POINT_FS_SRC)
    .expect("shader program")
    .ignore_warnings();
  let line_program = Program::<Semantics, (), ()>::from_strings(None, VS_SRC, None, LINE_FS_SRC)
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

            rebuild_tess = true;
          }
        }

        _ => ()
      }
    }

    if rebuild_tess {
      tess_curve = build_tess(&mut surface, &spline, Mode::LineStrip).expect("control point re-tessellation");
      tess_points = build_tess(&mut surface, &spline, Mode::Point).expect("control point re-tessellation");
    }

    // render
    let back_buffer = surface.back_buffer().unwrap();
    surface.pipeline_builder().pipeline(&back_buffer, [0., 0., 0., 0.], |_, mut shd_gate| {
      // lines
      shd_gate.shade(&line_program, |_, mut rdr_gate| {
        rdr_gate.render(RenderState::default(), |mut tess_gate| {
            tess_gate.render(&tess_curve);
        });
      });

      // points
      shd_gate.shade(&point_program, |_, mut rdr_gate| {
        rdr_gate.render(
          RenderState::default()
            .set_blending(Some((Equation::Additive, Factor::SrcAlpha, Factor::SrcAlphaComplement)))
            .set_depth_test(DepthTest::Off)
          , |mut tess_gate| {
            tess_gate.render(&tess_points);
        });
      });
    });

    surface.swap_buffers();
  }
}
