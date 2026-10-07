//! A closed triangle: the head of `<|--` (18 wide) and of `-->>` (8 wide).

use std::f64::consts::FRAC_PI_2;

use super::{Extremity, ExtremityFactory, manageround, rotated_polygon};
use crate::klimt::UDrawable;
use crate::klimt::geom::XPoint2D;
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::UGraphic;

/// PlantUML also takes a background colour, which every link decoration leaves unset.
pub(crate) struct ExtremityFactoryTriangle {
    x_wing: f64,
    y_aperture: f64,
    decoration_length: f64,
}

impl ExtremityFactoryTriangle {
    pub(crate) fn new(x_wing: f64, y_aperture: f64, decoration_length: f64) -> Self {
        Self {
            x_wing,
            y_aperture,
            decoration_length,
        }
    }
}

impl ExtremityFactory for ExtremityFactoryTriangle {
    fn create_udrawable(&self, p0: XPoint2D, angle: f64) -> Box<dyn Extremity> {
        let angle = manageround(angle - FRAC_PI_2);
        let points = [
            (0.0, 0.0),
            (-self.x_wing, -self.y_aperture),
            (-self.x_wing, self.y_aperture),
            (0.0, 0.0),
        ];
        Box::new(ExtremityTriangle {
            polygon: rotated_polygon(&points, angle + FRAC_PI_2, p0),
            decoration_length: self.decoration_length,
        })
    }
}

struct ExtremityTriangle {
    polygon: UShape,
    decoration_length: f64,
}

impl UDrawable for ExtremityTriangle {
    fn draw_u(&self, ug: &UGraphic) {
        ug.draw(&self.polygon);
    }
}

impl Extremity for ExtremityTriangle {
    fn decoration_length(&self) -> f64 {
        self.decoration_length
    }
}
