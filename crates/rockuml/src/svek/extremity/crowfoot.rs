//! The crow's foot of "many": `}`.

use std::f64::consts::FRAC_PI_2;

use super::{Extremity, ExtremityFactory, draw_line, manageround};
use crate::klimt::UDrawable;
use crate::klimt::affine::XAffineTransform;
use crate::klimt::geom::XPoint2D;
use crate::klimt::ugraphic::UGraphic;

pub(crate) struct ExtremityFactoryCrowfoot;

impl ExtremityFactory for ExtremityFactoryCrowfoot {
    fn create_udrawable(&self, p0: XPoint2D, angle: f64) -> Box<dyn Extremity> {
        Box::new(ExtremityCrowfoot::new(p0, angle - FRAC_PI_2))
    }
}

struct ExtremityCrowfoot {
    contact: XPoint2D,
    angle: f64,
}

impl ExtremityCrowfoot {
    fn new(p1: XPoint2D, angle: f64) -> Self {
        Self {
            contact: p1,
            angle: manageround(angle + FRAC_PI_2),
        }
    }
}

impl UDrawable for ExtremityCrowfoot {
    fn draw_u(&self, ug: &UGraphic) {
        const X_WING: f64 = 8.0;
        const Y_APERTURE: f64 = 8.0;
        let rotate = XAffineTransform::rotate_instance(self.angle);
        let middle = XPoint2D::new(0.0, 0.0);
        let left = XPoint2D::new(0.0, -Y_APERTURE).transform(&rotate);
        let base = XPoint2D::new(-X_WING, 0.0).transform(&rotate);
        let right = XPoint2D::new(0.0, Y_APERTURE).transform(&rotate);

        let (x, y) = (self.contact.x, self.contact.y);
        draw_line(ug, x, y, base, left);
        draw_line(ug, x, y, base, right);
        draw_line(ug, x, y, base, middle);
    }
}

impl Extremity for ExtremityCrowfoot {}
