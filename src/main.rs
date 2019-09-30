// This is currently a prototype. The current code is pretty naive, especially in terms of
// allocation of keys in splines. Some  work must be done to clean all that stuff.

mod editor;
mod vertex;

use luminance::blending::{Equation, Factor};
use luminance::context::GraphicsContext;
use luminance::render_state::RenderState;
use luminance::shader::program::Program;
use luminance_glfw::{Action, GlfwSurface, Key as GKey, MouseButton, Surface, WindowDim, WindowEvent, WindowOpt};
use splines::Interpolation;

use crate::editor::{Editor, Selection, ScreenPos};
use crate::vertex::Semantics;

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

  let mut editor = Editor::new(&mut surface);

  // misc
  let mut cursor_pos: Option<[f32; 2]> = None;
  let mut cursor_pressed_pos: Option<[f32; 2]> = None;
  let mut mouse_left_pressed = false;

  let point_program = Program::<Semantics, (), ()>::from_strings(None, POINT_VS_SRC, POINT_GS_SRC, POINT_FS_SRC)
    .expect("shader program")
    .ignore_warnings();
  let line_program = Program::<Semantics, (), ()>::from_strings(None, LINE_VS_SRC, None, LINE_FS_SRC)
    .expect("shader program")
    .ignore_warnings();


  'app: loop {
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

          if mouse_left_pressed {
            if let Some(selection) = *editor.selection() {
              let p = ScreenPos::new(xy[0], xy[1]);

              match selection {
                Selection::Key(i) => {
                  editor.move_key(i, p).unwrap();
                }

                Selection::Handle(i, _) => {
                  editor.move_handle(i, p).unwrap();
                }
              }
            }
          }
        }

        WindowEvent::MouseButton(MouseButton::Button1, Action::Release, _) => {
          if !editor.is_selecting() {
            if let Some([x, y]) = cursor_pos {
              if cursor_pressed_pos == cursor_pos {
                editor.add_point(ScreenPos::new(x, y), Interpolation::Cosine);
              }
            }
          }

          mouse_left_pressed = false;
          cursor_pressed_pos = None;

        }

        WindowEvent::MouseButton(MouseButton::Button1, Action::Press, _) => {
          mouse_left_pressed = true;
          cursor_pressed_pos = cursor_pos;

          // try to select something at the current cursor, if any
          if let Some([x, y]) = cursor_pos {
            let _ = editor.select(ScreenPos::new(x, y));
          }
        }

        WindowEvent::Key(GKey::Backspace, _, Action::Release, _) => {
          if let Some(i) = editor.selected_point() {
            let _ = editor.remove_point(i);
          }
        }

        WindowEvent::Key(GKey::Space, _, Action::Release, _) => {
          if let Some(i) = editor.selected_point() {
            let _ = editor.toggle_interpolation(i);
          }
        }

        _ => ()
      }
    }

    editor.rebuild_tess_if_needed(&mut surface).unwrap();

    // render
    let back_buffer = surface.back_buffer().unwrap();
    let render_state = RenderState::default()
      .set_blending(Some((Equation::Additive, Factor::SrcAlpha, Factor::SrcAlphaComplement)))
      .set_depth_test(None);

    surface.pipeline_builder().pipeline(&back_buffer, [0., 0., 0., 0.], |_, mut shd_gate| {
      // lines
      shd_gate.shade(&line_program, |_, mut rdr_gate| {
        rdr_gate.render(render_state, |mut tess_gate| {
            tess_gate.render(editor.lines());
        });
      });

      // points
      shd_gate.shade(&point_program, |_, mut rdr_gate| {
        rdr_gate.render(render_state, |mut tess_gate| {
            tess_gate.render(editor.points());
        });
      });
    });

    surface.swap_buffers();
  }
}
