//! A rectangle drawing is limited to, as pages of sequence diagrams are (PlantUML's `UClip`). Each output
//! format clips its own way: shapes are dropped or cut, never partly hidden.

use super::shape::USegment;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UClip {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl UClip {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    #[must_use]
    pub fn translate(self, dx: f64, dy: f64) -> Self {
        Self::new(self.x + dx, self.y + dy, self.width, self.height)
    }

    pub fn is_inside(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }

    /// Whether a path's bounds lie inside, the path drawn at (`x`, `y`).
    pub fn is_path_inside(&self, x: f64, y: f64, segments: &[USegment]) -> bool {
        let Some((min_x, min_y, max_x, max_y)) = path_bounds(segments) else {
            return false;
        };
        self.is_inside(x + min_x, y + min_y) && self.is_inside(x + max_x, y + max_y)
    }

    // Unlike `f64::clamp`, these never panic, whatever the clip's size.
    fn clipped_x(&self, x: f64) -> f64 {
        x.max(self.x).min(self.x + self.width)
    }

    fn clipped_y(&self, y: f64) -> f64 {
        y.max(self.y).min(self.y + self.height)
    }

    /// The part of a rectangle inside, as (x, y, width, height); the size may be negative.
    pub fn clipped_rectangle(
        &self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> (f64, f64, f64, f64) {
        let x1 = x.max(self.x);
        let y1 = y.max(self.y);
        let x2 = (x + width).min(self.x + self.width);
        let y2 = (y + height).min(self.y + self.height);
        (x1, y1, x2 - x1, y2 - y1)
    }

    /// The part of a horizontal or vertical line inside; other lines are kept whole or dropped.
    pub fn clipped_line(
        &self,
        (x1, y1): (f64, f64),
        (x2, y2): (f64, f64),
    ) -> Option<((f64, f64), (f64, f64))> {
        let inside1 = self.is_inside(x1, y1);
        let inside2 = self.is_inside(x2, y2);
        if inside1 && inside2 {
            return Some(((x1, y1), (x2, y2)));
        }
        if !inside1 && !inside2 {
            if x1 == x2 {
                let (new_y1, new_y2) = (self.clipped_y(y1), self.clipped_y(y2));
                if new_y1 != new_y2 {
                    return Some(((x1, new_y1), (x2, new_y2)));
                }
            }
            return None;
        }
        if y1 == y2 {
            return Some(((self.clipped_x(x1), y1), (self.clipped_x(x2), y2)));
        }
        if x1 == x2 {
            return Some(((x1, self.clipped_y(y1)), (x2, self.clipped_y(y2))));
        }
        None
    }
}

/// The bounds PlantUML gives a path: every point of its segments, but only the end of an arc.
pub(crate) fn path_bounds(segments: &[USegment]) -> Option<(f64, f64, f64, f64)> {
    segments
        .iter()
        .flat_map(|segment| match *segment {
            USegment::MoveTo(x, y) | USegment::LineTo(x, y) => vec![(x, y)],
            USegment::CubicTo { ctrl1, ctrl2, end } => vec![ctrl1, ctrl2, end],
            USegment::ArcTo { end, .. } => vec![end],
        })
        .fold(None, |bounds, (x, y)| {
            Some(match bounds {
                None => (x, y, x, y),
                Some((min_x, min_y, max_x, max_y)) => {
                    (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y))
                }
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn straight_lines_are_cut_others_kept_or_dropped() {
        let clip = UClip::new(0.0, 10.0, 100.0, 20.0);
        assert_eq!(
            clip.clipped_line((5.0, 0.0), (5.0, 50.0)),
            Some(((5.0, 10.0), (5.0, 30.0)))
        );
        assert_eq!(clip.clipped_line((5.0, 0.0), (5.0, 5.0)), None);
        assert_eq!(clip.clipped_line((0.0, 0.0), (50.0, 20.0)), None);
        assert_eq!(
            clip.clipped_line((-10.0, 15.0), (50.0, 15.0)),
            Some(((0.0, 15.0), (50.0, 15.0)))
        );
    }
}
