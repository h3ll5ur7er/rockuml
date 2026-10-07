//! "Zero or many": a circle behind a crow's foot, `}o`.

use std::f64::consts::FRAC_PI_2;

use super::{Extremity, ExtremityFactory, draw_line, manageround};
use crate::klimt::UDrawable;
use crate::klimt::affine::XAffineTransform;
use crate::klimt::geom::XPoint2D;
use crate::klimt::shape::{UEllipse, UShape};
use crate::klimt::ugraphic::UGraphic;

pub(crate) struct ExtremityFactoryCircleCrowfoot;

impl ExtremityFactory for ExtremityFactoryCircleCrowfoot {
    fn create_udrawable(&self, p0: XPoint2D, angle: f64) -> Box<dyn Extremity> {
        Box::new(ExtremityCircleCrowfoot::new(p0, angle - FRAC_PI_2))
    }
}

const RADIUS: f64 = 4.0;

struct ExtremityCircleCrowfoot {
    contact: XPoint2D,
    angle: f64,
}

impl ExtremityCircleCrowfoot {
    fn new(p1: XPoint2D, angle: f64) -> Self {
        Self {
            contact: p1,
            angle: manageround(angle + FRAC_PI_2),
        }
    }
}

impl UDrawable for ExtremityCircleCrowfoot {
    fn draw_u(&self, ug: &UGraphic) {
        const X_WING: f64 = 8.0;
        const Y_APERTURE: f64 = 6.0;
        let rotate = XAffineTransform::rotate_instance(self.angle);
        let middle = XPoint2D::new(0.0, 0.0);
        let left = XPoint2D::new(0.0, -Y_APERTURE).transform(&rotate);
        let base = XPoint2D::new(-X_WING, 0.0).transform(&rotate);
        let right = XPoint2D::new(0.0, Y_APERTURE).transform(&rotate);
        let circle_base = XPoint2D::new(-X_WING - RADIUS - 2.0, 0.0).transform(&rotate);

        let (x, y) = (self.contact.x, self.contact.y);
        draw_line(ug, x, y, base, left);
        draw_line(ug, x, y, base, right);
        draw_line(ug, x, y, base, middle);
        ug.translated(x + circle_base.x - RADIUS, y + circle_base.y - RADIUS)
            .draw(&UShape::Ellipse(UEllipse::new(2.0 * RADIUS, 2.0 * RADIUS)));
    }
}

impl Extremity for ExtremityCircleCrowfoot {
    fn decoration_length(&self) -> f64 {
        18.0
    }
}
