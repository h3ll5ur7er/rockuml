//! One wing of an arrowhead, `\\` above the link or `//` below it.

use super::{Extremity, ExtremityFactory, change_back, manageround};
use crate::klimt::UDrawable;
use crate::klimt::affine::XAffineTransform;
use crate::klimt::geom::XPoint2D;
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::UGraphic;

pub(crate) struct ExtremityFactoryHalfArrow {
    direction: i32,
}

impl ExtremityFactoryHalfArrow {
    /// `1` for the upper wing, `-1` for the lower one.
    pub(crate) fn new(direction: i32) -> Self {
        Self { direction }
    }
}

impl ExtremityFactory for ExtremityFactoryHalfArrow {
    fn create_udrawable(&self, p0: XPoint2D, angle: f64) -> Box<dyn Extremity> {
        Box::new(ExtremityHalfArrow::new(p0, angle, self.direction))
    }
}

struct ExtremityHalfArrow {
    line: UShape,
    other_line: UShape,
    contact: XPoint2D,
}

impl ExtremityHalfArrow {
    fn new(p0: XPoint2D, angle: f64, direction: i32) -> Self {
        const X_WING: f64 = 9.0;
        let y_aperture = f64::from(4 * direction);
        let rotate = XAffineTransform::rotate_instance(manageround(angle));
        let other = XPoint2D::new(-X_WING, -y_aperture).transform(&rotate);
        let other2 = XPoint2D::new(-8.0, 0.0).transform(&rotate);
        Self {
            line: UShape::Line {
                dx: other.x,
                dy: other.y,
            },
            other_line: UShape::Line {
                dx: other2.x,
                dy: other2.y,
            },
            contact: p0,
        }
    }
}

impl UDrawable for ExtremityHalfArrow {
    fn draw_u(&self, ug: &UGraphic) {
        let ug = change_back(ug).translated(self.contact.x, self.contact.y);
        ug.draw(&self.line);
        ug.draw(&self.other_line);
    }
}

impl Extremity for ExtremityHalfArrow {}
