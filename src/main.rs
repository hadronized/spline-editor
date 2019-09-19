use luminance::context::GraphicsContext;
use luminance::render_state::RenderState;
use luminance::shader::program::Program;
use luminance::tess::{Tess, TessBuilder};
use luminance_derive::{Semantics, Vertex};
use luminance_glfw::{Action, GlfwSurface, Key as GKey, MouseButton, Surface, WindowDim, WindowEvent, WindowOpt};
use splines::{Interpolation, Key};

#[derive(Copy, Clone, Debug, Semantics)]
pub enum Semantics {
  #[sem(name = "position", repr = "[f32; 2]", wrapper = "VPos")]
  Position,
}

#[derive(Clone, Debug, Vertex)]
#[vertex(sem = "Semantics")]
struct Vertex {
  position: VPos
}

/// Our control point type.
type CP = Key<f32, f32>;

/// Rebuild tessellation based on control points.
fn build_tess<C>(ctx: &mut C, cps: &[CP]) -> Result<Tess, String> where C: GraphicsContext {
  let vertices: Vec<_> = cps.iter().map(|cp| Vertex { position: VPos::new([cp.t, cp.value]) }).collect();
  TessBuilder::new(ctx)
    .add_vertices(vertices)
    .build()
    .map_err(|e| format!("{:?}", e))
}

const VS_SRC: &str = include_str!("vs.glsl");
const GS_SRC: &str = include_str!("gs.glsl");
const FS_SRC: &str = include_str!("fs.glsl");

fn main() {
  let mut surface = GlfwSurface::new(WindowDim::Windowed(800, 800), "spline editor", WindowOpt::default())
    .expect("create surface");

  // control points
  let mut cps: Vec<CP> = Vec::new();
  let mut cursor_pos: Option<[f32; 2]> = None;

  // tessellated curve
  let mut tess_curve = build_tess(&mut surface, &cps).expect("control point tessellation");

  let (program, _) = Program::<Semantics, (), ()>::from_strings(None, VS_SRC, GS_SRC, FS_SRC).expect("shader program");

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

            cps.push(Key::new(x, y, Interpolation::Cosine));
            rebuild_tess = true;
          }
        }

        _ => ()
      }
    }

    if rebuild_tess {
      tess_curve = build_tess(&mut surface, &cps).expect("control point re-tessellation");
    }

    // render
    let back_buffer = surface.back_buffer().unwrap();
    surface.pipeline_builder().pipeline(&back_buffer, [0., 0., 0., 0.], |_, mut shd_gate| {
      shd_gate.shade(&program, |_, mut rdr_gate| {
        rdr_gate.render(RenderState::default(), |mut tess_gate| {
          tess_gate.render(&tess_curve);
        });
      });
    });

    surface.swap_buffers();
  }
}
