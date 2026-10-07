//! "Zero or one": a circle behind a bar, `|o`, both as thick as the link.

use std::f64::consts::FRAC_PI_2;

use super::{Extremity, ExtremityFactory, draw_line, manageround};
use crate::klimt::UDrawable;
use crate::klimt::affine::XAffineTransform;
use crate::klimt::geom::XPoint2D;
use crate::klimt::shape::{UEllipse, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};

pub(crate) struct ExtremityFactoryCircleLine;

impl ExtremityFactory for ExtremityFactoryCircleLine {
    fn create_udrawable(&self, p0: XPoint2D, angle: f64) -> Box<dyn Extremity> {
        Box::new(ExtremityCircleLine::new(p0, angle - FRAC_PI_2))
    }
}

struct ExtremityCircleLine {
    contact: XPoint2D,
    angle: f64,
}

impl ExtremityCircleLine {
    fn new(p1: XPoint2D, angle: f64) -> Self {
        Self {
            contact: p1,
            angle: manageround(angle + FRAC_PI_2),
        }
    }
}

impl UDrawable for ExtremityCircleLine {
    fn draw_u(&self, ug: &UGraphic) {
        const X_WING: f64 = 4.0;
        let thickness = ug.param().stroke.thickness;
        let radius = 4.0 + thickness - 1.0;
        let line_height = 4.0 + thickness - 1.0;
        let rotate = XAffineTransform::rotate_instance(self.angle);
        let middle = XPoint2D::new(0.0, 0.0);
        let base = XPoint2D::new(-X_WING - radius - 3.0, 0.0).transform(&rotate);
        let circle_base = XPoint2D::new(-X_WING - radius - 3.0, 0.0).transform(&rotate);
        let line_top = XPoint2D::new(-X_WING, -line_height).transform(&rotate);
        let line_bottom = XPoint2D::new(-X_WING, line_height).transform(&rotate);

        let (x, y) = (self.contact.x, self.contact.y);
        draw_line(ug, x, y, base, middle);
        let stroke = UStroke::with_thickness(thickness);
        ug.translated(x + circle_base.x - radius, y + circle_base.y - radius)
            .with_stroke(stroke)
            .draw(&UShape::Ellipse(UEllipse::new(2.0 * radius, 2.0 * radius)));
        draw_line(&ug.with_stroke(stroke), x, y, line_top, line_bottom);
    }
}

impl Extremity for ExtremityCircleLine {
    fn decoration_length(&self) -> f64 {
        15.0
    }
}
