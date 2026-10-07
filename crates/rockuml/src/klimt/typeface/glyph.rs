//! Glyph outlines, for the characters PlantUML draws as shapes instead of text.

use resvg::tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Transform};
use ttf_parser::{Face, OutlineBuilder};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum GlyphSegment {
    MoveTo(f64, f64),
    LineTo(f64, f64),
    QuadTo {
        ctrl: (f64, f64),
        end: (f64, f64),
    },
    CubicTo {
        ctrl1: (f64, f64),
        ctrl2: (f64, f64),
        end: (f64, f64),
    },
    Close,
}

/// A glyph's outline at the font's size, around its origin on the baseline, with y going down.
pub(crate) struct GlyphOutline {
    pub segments: Vec<GlyphSegment>,
}

impl GlyphOutline {
    /// Java's font scaler hands outlines out in whole 64ths of a pixel, the on-curve points TrueType leaves
    /// implicit between two off-curve points included.
    pub(super) fn of(face: &Face, c: char, size: f64) -> Option<Self> {
        let glyph = face.glyph_index(c)?;
        let mut builder = Builder::default();
        face.outline_glyph(glyph, &mut builder);
        let scale = size / f64::from(face.units_per_em());
        let in_64ths = |(x, y): (f64, f64)| {
            let rounded = |units: f64| (units * scale * 64.0).round() as i64;
            (rounded(x), rounded(y))
        };
        let to_pixels = |(x, y): (i64, i64)| (x as f64 / 64.0, -(y as f64) / 64.0);
        let pixels = |point| to_pixels(in_64ths(point));
        let midpoint_in_64ths = |a, b| {
            let ((ax, ay), (bx, by)) = (in_64ths(a), in_64ths(b));
            to_pixels((i64::midpoint(ax, bx), i64::midpoint(ay, by)))
        };
        let in_units = &builder.segments;
        let segments = in_units
            .iter()
            .enumerate()
            .map(|(index, segment)| match *segment {
                GlyphSegment::MoveTo(x, y) => {
                    let (x, y) = pixels((x, y));
                    GlyphSegment::MoveTo(x, y)
                }
                GlyphSegment::LineTo(x, y) => {
                    let (x, y) = pixels((x, y));
                    GlyphSegment::LineTo(x, y)
                }
                GlyphSegment::QuadTo { ctrl, end } => {
                    let end = match in_units.get(index + 1) {
                        Some(&GlyphSegment::QuadTo { ctrl: next, .. })
                            if midpoint(ctrl, next) == end =>
                        {
                            midpoint_in_64ths(ctrl, next)
                        }
                        _ => pixels(end),
                    };
                    GlyphSegment::QuadTo {
                        ctrl: pixels(ctrl),
                        end,
                    }
                }
                GlyphSegment::CubicTo { ctrl1, ctrl2, end } => GlyphSegment::CubicTo {
                    ctrl1: pixels(ctrl1),
                    ctrl2: pixels(ctrl2),
                    end: pixels(end),
                },
                GlyphSegment::Close => GlyphSegment::Close,
            })
            .collect();
        Some(Self { segments })
    }

    /// PlantUML's `UnusedSpace`: the centre of the smallest circle around the pixels the glyph covers, to a
    /// quarter pixel. The glyph is drawn without antialiasing on its baseline in a 40-pixel square.
    pub(crate) fn center(&self) -> (f64, f64) {
        const HALF_SIZE: i32 = 20;
        let points = self.pixels(HALF_SIZE);
        let (Some(min_i), Some(max_i), Some(min_j), Some(max_j)) = (
            points.iter().map(|&(i, _)| i).min(),
            points.iter().map(|&(i, _)| i).max(),
            points.iter().map(|&(_, j)| j).min(),
            points.iter().map(|&(_, j)| j).max(),
        ) else {
            return (0.0, 0.0);
        };
        let biggest_distance_sq = |x: f64, y: f64| {
            points
                .iter()
                .map(|&(i, j)| {
                    let (dx, dy) = (x - f64::from(i), y - f64::from(j));
                    dx * dx + dy * dy
                })
                .fold(0.0, f64::max)
        };
        let mut min = f64::MAX;
        let mut center = (0.0, 0.0);
        for i in min_i * 4..=max_i * 4 {
            for j in min_j * 4..max_j * 4 {
                let (x, y) = (f64::from(i) / 4.0, f64::from(j) / 4.0);
                let distance_sq = biggest_distance_sq(x, y);
                if distance_sq < min {
                    min = distance_sq;
                    center = (x - f64::from(HALF_SIZE), y - f64::from(HALF_SIZE));
                }
            }
        }
        center
    }

    /// The pixels the glyph covers when drawn on its baseline at (`half_size`, `half_size`).
    fn pixels(&self, half_size: i32) -> Vec<(i32, i32)> {
        let width = 2 * half_size;
        let mut pixmap = Pixmap::new(width as u32, width as u32).expect("a small square");
        if let Some(path) = self.tiny_skia_path() {
            let mut paint = Paint::default();
            paint.set_color_rgba8(255, 255, 255, 255);
            paint.anti_alias = false;
            let origin = half_size as f32;
            pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::from_translate(origin, origin),
                None,
            );
        }
        (0..width)
            .flat_map(|i| (0..width).map(move |j| (i, j)))
            .filter(|&(i, j)| {
                pixmap
                    .pixel(i as u32, j as u32)
                    .is_some_and(|pixel| pixel.alpha() > 0)
            })
            .collect()
    }

    fn tiny_skia_path(&self) -> Option<resvg::tiny_skia::Path> {
        let mut path = PathBuilder::new();
        let point = |(x, y): (f64, f64)| (x as f32, y as f32);
        for segment in &self.segments {
            match *segment {
                GlyphSegment::MoveTo(x, y) => path.move_to(x as f32, y as f32),
                GlyphSegment::LineTo(x, y) => path.line_to(x as f32, y as f32),
                GlyphSegment::QuadTo { ctrl, end } => {
                    let ((x1, y1), (x, y)) = (point(ctrl), point(end));
                    path.quad_to(x1, y1, x, y);
                }
                GlyphSegment::CubicTo { ctrl1, ctrl2, end } => {
                    let ((x1, y1), (x2, y2), (x, y)) = (point(ctrl1), point(ctrl2), point(end));
                    path.cubic_to(x1, y1, x2, y2, x, y);
                }
                GlyphSegment::Close => path.close(),
            }
        }
        path.finish()
    }
}

fn midpoint(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (f64::midpoint(a.0, b.0), f64::midpoint(a.1, b.1))
}

/// Collects the outline in font units.
#[derive(Default)]
struct Builder {
    segments: Vec<GlyphSegment>,
}

fn units(x: f32, y: f32) -> (f64, f64) {
    (f64::from(x), f64::from(y))
}

impl OutlineBuilder for Builder {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = units(x, y);
        self.segments.push(GlyphSegment::MoveTo(x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = units(x, y);
        self.segments.push(GlyphSegment::LineTo(x, y));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.segments.push(GlyphSegment::QuadTo {
            ctrl: units(x1, y1),
            end: units(x, y),
        });
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.segments.push(GlyphSegment::CubicTo {
            ctrl1: units(x1, y1),
            ctrl2: units(x2, y2),
            end: units(x, y),
        });
    }

    fn close(&mut self) {
        self.segments.push(GlyphSegment::Close);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klimt::font::{UFont, UFontFace};
    use crate::klimt::typeface::FontRegistry;

    #[test]
    fn the_center_is_that_of_the_smallest_circle_around_the_pixels() {
        let block = GlyphOutline {
            segments: vec![
                GlyphSegment::MoveTo(0.0, -10.0),
                GlyphSegment::LineTo(6.0, -10.0),
                GlyphSegment::LineTo(6.0, 0.0),
                GlyphSegment::LineTo(0.0, 0.0),
                GlyphSegment::Close,
            ],
        };
        assert_eq!(block.center(), (2.5, -5.5));
    }

    #[test]
    fn nothing_drawn_centers_on_the_origin() {
        let empty = GlyphOutline {
            segments: Vec::new(),
        };
        assert_eq!(empty.center(), (0.0, 0.0));
    }

    #[test]
    fn outlines_come_in_64ths_of_a_pixel() {
        let font = UFont::new("Monospaced", UFontFace::BOLD, 17);
        let outline = FontRegistry::default().glyph_outline(&font, 'C').unwrap();
        let on_grid = |(x, y): (f64, f64)| (x * 64.0).fract() == 0.0 && (y * 64.0).fract() == 0.0;
        assert!(outline.segments.iter().all(|segment| match *segment {
            GlyphSegment::MoveTo(x, y) | GlyphSegment::LineTo(x, y) => on_grid((x, y)),
            GlyphSegment::QuadTo { ctrl, end } => on_grid(ctrl) && on_grid(end),
            GlyphSegment::CubicTo { ctrl1, ctrl2, end } => {
                on_grid(ctrl1) && on_grid(ctrl2) && on_grid(end)
            }
            GlyphSegment::Close => true,
        }));
        assert!(outline.segments.len() > 10);
    }
}
