//! The open arrowhead of `-->`.

use super::{Extremity, ExtremityFactory, change_back, manageround, rotated_polygon};
use crate::klimt::UDrawable;
use crate::klimt::geom::XPoint2D;
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::UGraphic;

pub(crate) struct ExtremityFactoryArrow;

impl ExtremityFactory for ExtremityFactoryArrow {
    fn create_udrawable(&self, p0: XPoint2D, angle: f64) -> Box<dyn Extremity> {
        Box::new(ExtremityArrow::new(p0, angle))
    }
}

struct ExtremityArrow {
    polygon: UShape,
}

impl ExtremityArrow {
    fn new(p0: XPoint2D, angle: f64) -> Self {
        const X_WING: f64 = 9.0;
        const Y_APERTURE: f64 = 4.0;
        const X_CONTACT: f64 = 5.0;
        let points = [
            (0.0, 0.0),
            (-X_WING, -Y_APERTURE),
            (-X_CONTACT, 0.0),
            (-X_WING, Y_APERTURE),
            (0.0, 0.0),
        ];
        Self {
            polygon: rotated_polygon(&points, manageround(angle), p0),
        }
    }
}

impl UDrawable for ExtremityArrow {
    fn draw_u(&self, ug: &UGraphic) {
        change_back(ug).draw(&self.polygon);
    }
}

impl Extremity for ExtremityArrow {
    fn decoration_length(&self) -> f64 {
        5.0
    }
}
