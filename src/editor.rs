use cgmath::Vector2;
use luminance::context::GraphicsContext;
use luminance::tess::{Mode, Tess, TessError, TessBuilder};
use splines::{Interpolation, Key, Spline};

use crate::vertex::{LineVertex, VColor, VRadius, VPos, PointVertex};

const DELTA_T: f32 = 0.01;
const POINT_SELECTION_DIST: f32 = DELTA_T * 8.;

/// Position on screen.
type ScreenPos = Vector2<f32>;

/// Editor object.
struct Editor {
  // The actual spline the user is editing.
  spline: Spline<f32, ScreenPos>,
  // Currently selected content.
  selection: Option<Selection>,
  // List of display points.
  points: Tess,
  // List of lines.
  lines: Tess,
  // Hint to know whether tessellations should be rebuilt.
  rebuild_tess: bool,
}

impl Editor {
  /// Rebuild tessellation based on control points for lines.
  fn build_lines<C>(&mut self, ctx: &mut C) -> Result<(), EditorError> where C: GraphicsContext {
    let mut vertices = Vec::new();
    let keys = self.spline.keys();

    if !keys.is_empty() {
      let up_t = keys.last().unwrap().t;
      let mut t = keys[0].t;

      while t <= up_t {
        let p = self.spline.clamped_sample(t).unwrap();
        vertices.push(LineVertex::new(VPos::new(p.into())));
        t += DELTA_T;
      }
    }

    self.lines = TessBuilder::new(ctx)
      .set_mode(Mode::LineStrip)
      .add_vertices(vertices)
      .build()
      .map_err(EditorError::TessError)?;

    Ok(())
  }

  /// Rebuild tessellation based on control points for points.
  fn build_points<C>(&mut self, ctx: &mut C) -> Result<(), EditorError> where C: GraphicsContext {
    let mut vertices = Vec::new();
    let keys = self.spline.keys();

    let mut specials = Vec::new();

    if !keys.is_empty() {
      for (i, cp) in keys.iter().enumerate() {
        let mut vertex = PointVertex::new(
          VPos::new(cp.value.into()),
          VColor::new([0.5, 0.5, 1.]),
          VRadius::new(0.025 / 2.),
        );

        if let Some(Selection::Key(i_sel)) = self.selection {
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

          if let Some(Selection::Handle(i_sel, _)) = self.selection {
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

    self.points = TessBuilder::new(ctx)
      .set_mode(Mode::Point)
      .add_vertices(vertices)
      .build()
      .map_err(EditorError::TessError)?;

    Ok(())
  }

  /// Rebuild tessellation.
  fn rebuild_tess<C>(
    &mut self,
    surface: &mut C
  ) -> Result<(), EditorError>
    where
      C: GraphicsContext {
    self.build_points(surface)?;
    self.build_lines(surface)
  }

  /// Move a point.
  fn move_key(&mut self, index: usize, p: ScreenPos) -> Result<(), EditorError> {
    let key = self.spline.get_mut(index).ok_or_else(|| EditorError::UnknownKey(index))?;
    *key.value = p;
    Ok(())
  }

  /// Move a handle of a point.
  fn move_handle(&mut self, index: usize, p: ScreenPos) -> Result<(), EditorError> {
    let key = self.spline.get_mut(index).ok_or_else(|| EditorError::UnknownKey(index))?;

    if let Interpolation::Bezier(ref mut handle) = *key.interpolation {
      *handle = p;
      Ok(())
    } else {
      Err(EditorError::WrongInterpolationAssumed(index))
    }
  }

  /// Add a new point.
  fn add_point(&mut self, p: ScreenPos, interpolation: Interpolation<f32, ScreenPos>) {
    self.selection = None;
    self.spline.add(Key::new(p[0], p, interpolation));
    self.rebuild_tess = true;
  }

  /// Remove a point.
  fn remove_point(&mut self, index: usize) -> Result<Key<f32, ScreenPos>, EditorError> {
    self.spline.remove(index).ok_or_else(|| EditorError::UnknownKey(index))
  }

  /// Try to select some content at the given position. The selected content is returned if any.
  fn select(&mut self, cursor_pos: ScreenPos) -> Option<Selection> {
    let [x, y]: [f32; 2] = cursor_pos.into();
    let mut found = None;

    // we want to select a point; check if any is nearby
    for (i, p) in self.spline.keys().iter().enumerate() {
      let [px, py]: [f32; 2] = p.value.into();
      let dist = ((x - px).powf(2.) + (y - py).powf(2.)).sqrt();

      if dist <= POINT_SELECTION_DIST {
        // try to select a key first
        match found {
          Some((_, prev_dist)) if dist < prev_dist => {
            found = Some((Selection::Key(i), dist));
          }

          None => {
            found = Some((Selection::Key(i), dist));
          }

          _ => ()
        }
      } else if let Interpolation::Bezier(ref handle) = p.interpolation {
        // try to select a handle
        let [px, py]: [f32; 2] = (*handle).into();
        let dist = ((x - px).powf(2.) + (y - py).powf(2.)).sqrt();

        if dist <= POINT_SELECTION_DIST {
          match found {
            Some((_, prev_dist)) if dist < prev_dist => {
              found = Some((Selection::Handle(i, HandleSelection::Own), dist));
            }

            None => {
              found = Some((Selection::Handle(i, HandleSelection::Own), dist));
            }

            _ => ()
          }
        }
      }
    }

    self.selection = found.map(|(selection, _)| {
      match selection {
        Selection::Key(i) => {
          println!("selecting point {}", i);
        }

        Selection::Handle(i, _) => {
          println!("selecting handle {}", i);
        }
      }

      selection
    });

    self.rebuild_tess = true;

    self.selection
  }

  /// Toggle the interpolation of a key to something else.
  fn toggle_interpolation(&mut self, index: usize) -> Result<(), EditorError> {
    let key = self.spline.get_mut(index).ok_or_else(|| EditorError::UnknownKey(index))?;
    let prev = *key.interpolation;
    *key.interpolation = Self::cycle_interpolation(*key.value, prev);

    println!("toggling interpolation for key {}; {:?} -> {:?}", index, prev, key.interpolation);
    Ok(())
  }

  /// Switch interpolation back and forth between modes.
  fn cycle_interpolation(p: ScreenPos, i: Interpolation<f32, ScreenPos>) -> Interpolation<f32, ScreenPos> {
    match i {
      Interpolation::Step(_) => Interpolation::Linear,
      Interpolation::Linear => Interpolation::Cosine,
      Interpolation::Cosine => Interpolation::Bezier(p + ScreenPos::new(0.1, 0.1)),
      Interpolation::Bezier(_) => Interpolation::Step(0.5),
      _ => i
    }
  }
}

/// Possible errors that might occur while using the editor.
pub enum EditorError {
  /// Unknown key index (i.e. likely out of bounds).
  UnknownKey(usize),
  /// Interpolation assumed is not the one the key is having.
  WrongInterpolationAssumed(usize),
  /// Error while rebuilding tessellation.
  TessError(TessError),
}

/// A selection. It can either be a control point (Key) or a handle for a Bézier curve. In case
/// of a handle, we either select the “real” handle or its mirrored sibling.
#[derive(Clone, Copy, Debug)]
pub enum Selection {
  /// A selected control point.
  Key(usize),
  /// A selected handle.
  Handle(usize, HandleSelection)
}

/// Part of handle being selected.
#[derive(Clone, Copy, Debug)]
pub enum HandleSelection {
  /// The actual handle of the control point.
  Own,
  /// Mirror handle of the control point.
  Mirror
}
