//! Measures a drawing by drawing it: a surface that only records how far shapes reach (PlantUML's
//! `LimitFinder`).

use std::cell::RefCell;
use std::rc::Rc;

use super::font::StringBounder;
use super::geom::{MinMax, UTranslate};
use super::shape::{USegment, UShape};
use super::ugraphic::{UGraphic, UGraphicBackend, UParam};
use crate::color::HColor;

/// Polygons count wider than they are, as in PlantUML.
const HACK_X_FOR_POLYGON: f64 = 10.0;

pub(crate) struct LimitFinder {
    string_bounder: Rc<dyn StringBounder>,
    min_max: MinMax,
}

impl LimitFinder {
    /// A surface measuring what is drawn on it, from the origin on.
    pub(crate) fn surface(
        string_bounder: Rc<dyn StringBounder>,
    ) -> (UGraphic, Rc<RefCell<LimitFinder>>) {
        let finder = Rc::new(RefCell::new(LimitFinder {
            string_bounder: string_bounder.clone(),
            min_max: MinMax::from_origin(),
        }));
        let ug = UGraphic::new(finder.clone(), string_bounder, HColor::WHITE);
        (ug, finder)
    }

    pub(crate) fn min_max(&self) -> MinMax {
        self.min_max
    }

    fn add_point(&mut self, x: f64, y: f64) {
        self.min_max = self.min_max.add_point(x, y);
    }

    /// The bounding box of points, empty ones adding nothing.
    fn add_bounds(
        &mut self,
        x: f64,
        y: f64,
        points: impl IntoIterator<Item = (f64, f64)>,
        extra_x: f64,
    ) {
        let mut bounds: Option<(f64, f64, f64, f64)> = None;
        for (px, py) in points {
            bounds = Some(match bounds {
                None => (px, py, px, py),
                Some((min_x, min_y, max_x, max_y)) => {
                    (min_x.min(px), min_y.min(py), max_x.max(px), max_y.max(py))
                }
            });
        }
        if let Some((min_x, min_y, max_x, max_y)) = bounds {
            self.add_point(x + min_x - extra_x, y + min_y);
            self.add_point(x + max_x + extra_x, y + max_y);
        }
    }
}

/// The points PlantUML bounds a path by: every point of a segment, but only the end of an arc.
fn path_points(segments: &[USegment]) -> Vec<(f64, f64)> {
    segments
        .iter()
        .flat_map(|segment| match *segment {
            USegment::MoveTo(x, y) | USegment::LineTo(x, y) => vec![(x, y)],
            USegment::CubicTo { ctrl1, ctrl2, end } => vec![ctrl1, ctrl2, end],
            USegment::ArcTo { end, .. } => vec![end],
        })
        .collect()
}

impl UGraphicBackend for LimitFinder {
    fn draw(&mut self, shape: &UShape, at: UTranslate, _param: &UParam) {
        let (x, y) = (at.dx, at.dy);
        match shape {
            UShape::Text(text) => {
                let dimension = self
                    .string_bounder
                    .calculate_dimension(&text.font.font(), &text.text);
                let top = y - (dimension.height - 1.5);
                self.add_point(x, top);
                self.add_point(x, top + dimension.height);
                self.add_point(x + dimension.width, top);
                self.add_point(x + dimension.width, top + dimension.height);
            }
            UShape::Line { dx, dy } => {
                self.add_point(x, y);
                self.add_point(x + dx, y + dy);
            }
            UShape::Ellipse(ellipse) => {
                self.add_point(x, y);
                self.add_point(x + ellipse.width - 1.0, y + ellipse.height - 1.0);
            }
            UShape::Rectangle(rectangle) => {
                self.add_point(x - 1.0, y - 1.0);
                self.add_point(x + rectangle.width - 1.0, y + rectangle.height - 1.0);
            }
            UShape::Polygon(points) => {
                self.add_bounds(x, y, points.iter().copied(), HACK_X_FOR_POLYGON);
            }
            UShape::Path(segments) => self.add_bounds(x, y, path_points(segments), 0.0),
            UShape::Image(image) => {
                self.add_point(x, y);
                self.add_point(x + image.width - 1.0, y + image.height - 1.0);
            }
            UShape::Empty(dimension) => {
                self.add_point(x, y);
                self.add_point(x + dimension.width, y + dimension.height);
            }
            UShape::HorizontalLine => {}
        }
    }
}
