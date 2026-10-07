//! "One or many": a bar behind a crow's foot, `}|`.

use std::f64::consts::FRAC_PI_2;

use super::{Extremity, ExtremityFactory, draw_line, manageround};
use crate::klimt::UDrawable;
use crate::klimt::affine::XAffineTransform;
use crate::klimt::geom::XPoint2D;
use crate::klimt::ugraphic::UGraphic;

pub(crate) struct ExtremityFactoryLineCrowfoot;

impl ExtremityFactory for ExtremityFactoryLineCrowfoot {
    fn create_udrawable(&self, p0: XPoint2D, angle: f64) -> Box<dyn Extremity> {
        Box::new(ExtremityLineCrowfoot::new(p0, angle - FRAC_PI_2))
    }
}

const LINE_HEIGHT: f64 = 4.0;

struct ExtremityLineCrowfoot {
    contact: XPoint2D,
    angle: f64,
}

impl ExtremityLineCrowfoot {
    fn new(p1: XPoint2D, angle: f64) -> Self {
        Self {
            contact: p1,
            angle: manageround(angle + FRAC_PI_2),
        }
    }
}

impl UDrawable for ExtremityLineCrowfoot {
    fn draw_u(&self, ug: &UGraphic) {
        const X_WING: f64 = 8.0;
        const Y_APERTURE: f64 = 6.0;
        let rotate = XAffineTransform::rotate_instance(self.angle);
        let middle = XPoint2D::new(0.0, 0.0);
        let left = XPoint2D::new(0.0, -Y_APERTURE).transform(&rotate);
        let base = XPoint2D::new(-X_WING, 0.0).transform(&rotate);
        let right = XPoint2D::new(0.0, Y_APERTURE).transform(&rotate);
        let line_top = XPoint2D::new(-X_WING - 2.0, -LINE_HEIGHT).transform(&rotate);
        let line_bottom = XPoint2D::new(-X_WING - 2.0, LINE_HEIGHT).transform(&rotate);

        let (x, y) = (self.contact.x, self.contact.y);
        draw_line(ug, x, y, base, left);
        draw_line(ug, x, y, base, right);
        draw_line(ug, x, y, base, middle);
        draw_line(ug, x, y, line_top, line_bottom);
    }
}

impl Extremity for ExtremityLineCrowfoot {}
