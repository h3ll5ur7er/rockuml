//! "Exactly one": two bars across the link, `||`.

use std::f64::consts::FRAC_PI_2;

use super::{Extremity, ExtremityFactory, draw_line, manageround};
use crate::klimt::UDrawable;
use crate::klimt::affine::XAffineTransform;
use crate::klimt::geom::XPoint2D;
use crate::klimt::ugraphic::UGraphic;

pub(crate) struct ExtremityFactoryDoubleLine;

impl ExtremityFactory for ExtremityFactoryDoubleLine {
    fn create_udrawable(&self, p0: XPoint2D, angle: f64) -> Box<dyn Extremity> {
        Box::new(ExtremityDoubleLine::new(p0, angle - FRAC_PI_2))
    }
}

const LINE_HEIGHT: f64 = 4.0;

struct ExtremityDoubleLine {
    contact: XPoint2D,
    angle: f64,
}

impl ExtremityDoubleLine {
    fn new(p1: XPoint2D, angle: f64) -> Self {
        Self {
            contact: p1,
            angle: manageround(angle + FRAC_PI_2),
        }
    }
}

impl UDrawable for ExtremityDoubleLine {
    fn draw_u(&self, ug: &UGraphic) {
        const X_WING: f64 = 4.0;
        let rotate = XAffineTransform::rotate_instance(self.angle);
        let first_line_top = XPoint2D::new(-X_WING, -LINE_HEIGHT).transform(&rotate);
        let first_line_bottom = XPoint2D::new(-X_WING, LINE_HEIGHT).transform(&rotate);
        let second_line_top = XPoint2D::new(-X_WING - 3.0, -LINE_HEIGHT).transform(&rotate);
        let second_line_bottom = XPoint2D::new(-X_WING - 3.0, LINE_HEIGHT).transform(&rotate);
        let middle = XPoint2D::new(0.0, 0.0).transform(&rotate);
        let base = XPoint2D::new(-X_WING - 4.0, 0.0).transform(&rotate);

        let (x, y) = (self.contact.x, self.contact.y);
        draw_line(ug, x, y, first_line_top, first_line_bottom);
        draw_line(ug, x, y, second_line_top, second_line_bottom);
        draw_line(ug, x, y, base, middle);
    }
}

impl Extremity for ExtremityDoubleLine {}
