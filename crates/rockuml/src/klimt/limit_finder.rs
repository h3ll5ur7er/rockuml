//! Measures a drawing by drawing it: a surface that only records how far shapes reach (PlantUML's
//! `LimitFinder`).

use std::cell::RefCell;
use std::rc::Rc;

use super::TextBlock;
use super::clip::{UClip, path_bounds};
use super::font::StringBounder;
use super::geom::{MinMax, UTranslate};
use super::shape::{UPath, UShape};
use super::ugraphic::{UGraphic, UGraphicBackend, UParam};
use crate::color::HColor;

/// Polygons count wider than they are, as in PlantUML.
const HACK_X_FOR_POLYGON: f64 = 10.0;

pub(crate) struct LimitFinder {
    string_bounder: Rc<dyn StringBounder>,
    min_max: MinMax,
    /// The clip of the shape being measured: points outside it do not count.
    clip: Option<UClip>,
}

impl LimitFinder {
    /// A surface measuring what is drawn on it, starting from `min_max`.
    pub(crate) fn surface(
        string_bounder: Rc<dyn StringBounder>,
        min_max: MinMax,
    ) -> (UGraphic, Rc<RefCell<LimitFinder>>) {
        let finder = Rc::new(RefCell::new(LimitFinder {
            string_bounder: string_bounder.clone(),
            min_max,
            clip: None,
        }));
        let ug = UGraphic::new(finder.clone(), string_bounder, HColor::WHITE);
        (ug, finder)
    }

    /// What was drawn, or the origin alone when nothing was.
    pub(crate) fn min_max(&self) -> MinMax {
        if self.min_max.min_x() == f64::MAX {
            return MinMax::from_origin();
        }
        self.min_max
    }

    /// How far a block reaches when drawn, the origin not counting (`TextBlockUtils.getMinMax`).
    pub(crate) fn min_max_of(
        block: &dyn TextBlock,
        string_bounder: Rc<dyn StringBounder>,
    ) -> MinMax {
        let (ug, finder) = Self::surface(string_bounder, MinMax::empty());
        block.draw_u(&ug);
        finder.borrow().min_max()
    }

    fn add_point(&mut self, x: f64, y: f64) {
        if self.clip.is_none_or(|clip| clip.is_inside(x, y)) {
            self.min_max = self.min_max.add_point(x, y);
        }
    }
}

impl UGraphicBackend for LimitFinder {
    fn draw(&mut self, shape: &UShape, at: UTranslate, param: &UParam) {
        self.clip = param.clip;
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
            UShape::Polygon(polygon) => {
                if !polygon.points().is_empty() {
                    let bounds = polygon.min_max();
                    self.add_point(x + bounds.min_x() - HACK_X_FOR_POLYGON, y + bounds.min_y());
                    self.add_point(x + bounds.max_x() + HACK_X_FOR_POLYGON, y + bounds.max_y());
                }
            }
            UShape::Path(UPath { segments, .. }) => {
                if let Some((min_x, min_y, max_x, max_y)) = path_bounds(segments) {
                    self.add_point(x + min_x, y + min_y);
                    self.add_point(x + max_x, y + max_y);
                }
            }
            UShape::Image(image) => {
                self.add_point(x, y);
                self.add_point(x + image.width() - 1.0, y + image.height() - 1.0);
            }
            UShape::ImageSvg(image) => {
                self.add_point(x, y);
                self.add_point(x + image.width() - 1.0, y + image.height() - 1.0);
            }
            UShape::Empty(dimension) => {
                self.add_point(x, y);
                self.add_point(x + dimension.width, y + dimension.height);
            }
            // The circle around a centred character already bounds it.
            UShape::HorizontalLine
            | UShape::SpecialText { .. }
            | UShape::CenteredText(_)
            | UShape::CenteredCharacter(_)
            | UShape::Comment(_) => {}
        }
    }
}
