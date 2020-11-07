// This is currently a prototype. The current code is pretty naive, especially in terms of
// allocation of keys in splines. Some  work must be done to clean all that stuff.

mod editor;
mod vertex;

use crate::{
  editor::{Editor, ScreenPos, Selection},
  vertex::Semantics,
};
use glfw::{Action, Context as _, Key, MouseButton, WindowEvent};
use luminance::{
  blending::{Blending, Equation, Factor},
  context::GraphicsContext,
  pipeline::PipelineState,
  render_state::RenderState,
};
use luminance_glfw::GlfwSurface;
use luminance_windowing::WindowOpt;
use splines::Interpolation;

const LINE_VS_SRC: &str = include_str!("vs.glsl");
const LINE_FS_SRC: &str = include_str!("fs.glsl");
const POINT_VS_SRC: &str = include_str!("point_vs.glsl");
const POINT_GS_SRC: &str = include_str!("point_gs.glsl");
const POINT_FS_SRC: &str = include_str!("point_fs.glsl");

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 800;

fn main() {
  let mut surface =
    GlfwSurface::new_gl33("spline editor", WindowOpt::default()).expect("create surface");

  let mut editor = Editor::new(&mut surface);

  // misc
  let mut cursor_pos: Option<[f32; 2]> = None;
  let mut cursor_pressed_pos: Option<[f32; 2]> = None;
  let mut mouse_left_pressed = false;

  let mut point_program = surface
    .new_shader_program::<Semantics, (), ()>()
    .from_strings(POINT_VS_SRC, None, POINT_GS_SRC, POINT_FS_SRC)
    .expect("shader program")
    .ignore_warnings();

  let mut line_program = surface
    .new_shader_program::<Semantics, (), ()>()
    .from_strings(LINE_VS_SRC, None, None, LINE_FS_SRC)
    .expect("shader program")
    .ignore_warnings();

  'app: loop {
    // event handling
    surface.window.glfw.poll_events();
    for (_, event) in glfw::flush_messages(&surface.events_rx) {
      match event {
        WindowEvent::Close | WindowEvent::Key(Key::Escape, _, Action::Release, _) => break 'app,

        WindowEvent::FramebufferSize(w, h) => {
          println!("new framebuffer dimensions: {}×{}", w, h);
        }

        WindowEvent::CursorPos(x, y) => {
          let xy = [
            x as f32 / WINDOW_WIDTH as f32,
            1. - 2. * y as f32 / WINDOW_HEIGHT as f32,
          ];
          cursor_pos = Some(xy);

          if mouse_left_pressed {
            if let Some(selection) = *editor.selection() {
              let p = ScreenPos::new(xy[0], xy[1]);

              match selection {
                Selection::Key(i) => {
                  editor.move_key(i, p).unwrap();
                }

                Selection::Handle(i, h) => {
                  editor.move_handle(i, p, h).unwrap();
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

        WindowEvent::MouseButton(MouseButton::Button2, Action::Release, _) => {
          editor.deselect();
        }

        WindowEvent::Key(Key::Backspace, _, Action::Release, _) => {
          if let Some(i) = editor.selected_point() {
            let _ = editor.remove_point(i);
          }
        }

        WindowEvent::Key(Key::Space, _, Action::Release, _) => {
          if let Some(i) = editor.selected_point() {
            let _ = editor.toggle_interpolation(i);
          }
        }

        _ => (),
      }
    }

    editor.rebuild_tess_if_needed(&mut surface).unwrap();

    // render
    let back_buffer = surface.back_buffer().unwrap();
    let pipeline_state = PipelineState::default();
    let render_state = RenderState::default()
      .set_blending(Blending {
        equation: Equation::Additive,
        src: Factor::SrcAlpha,
        dst: Factor::SrcAlphaComplement,
      })
      .set_depth_test(None);

    let render = surface
      .new_pipeline_gate()
      .pipeline(&back_buffer, &pipeline_state, |_, mut shd_gate| {
        // lines
        shd_gate.shade(&mut line_program, |_, _, mut rdr_gate| {
          rdr_gate.render(&render_state, |mut tess_gate| {
            tess_gate.render(editor.lines())
          })
        })?;

        // points
        shd_gate.shade(&mut point_program, |_, _, mut rdr_gate| {
          rdr_gate.render(&render_state, |mut tess_gate| {
            tess_gate.render(editor.points())
          })
        })
      })
      .assume();

    if render.is_ok() {
      surface.window.swap_buffers();
    } else {
      break;
    }
  }
}
