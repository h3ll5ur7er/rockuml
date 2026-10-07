//! A crossed circle centred on the end of the link.

use std::f64::consts::PI;

use super::{Extremity, ExtremityFactory, draw_line, point_on_circle};
use crate::color::HColor;
use crate::klimt::UDrawable;
use crate::klimt::geom::XPoint2D;
use crate::klimt::shape::{UEllipse, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};

pub(crate) struct ExtremityFactoryCircleCross {
    background_color: HColor,
}

impl ExtremityFactoryCircleCross {
    pub(crate) fn new(background_color: HColor) -> Self {
        Self { background_color }
    }
}

impl ExtremityFactory for ExtremityFactoryCircleCross {
    fn create_udrawable(&self, p0: XPoint2D, _angle: f64) -> Box<dyn Extremity> {
        Box::new(ExtremityCircleCross {
            px: p0.x - RADIUS,
            py: p0.y - RADIUS,
            dest: p0,
            background_color: self.background_color.clone(),
        })
    }
}

const RADIUS: f64 = 7.0;

struct ExtremityCircleCross {
    px: f64,
    py: f64,
    dest: XPoint2D,
    background_color: HColor,
}

impl ExtremityCircleCross {
    fn point_on_circle(&self, angle: f64) -> XPoint2D {
        point_on_circle(self.px, self.py, RADIUS, angle)
    }
}

impl UDrawable for ExtremityCircleCross {
    fn draw_u(&self, ug: &UGraphic) {
        let ug = ug.with_backcolor(self.background_color.clone());
        ug.with_stroke(UStroke::with_thickness(1.5))
            .translated(self.dest.x - RADIUS, self.dest.y - RADIUS)
            .draw(&UShape::Ellipse(UEllipse::new(RADIUS * 2.0, RADIUS * 2.0)));
        draw_line(
            &ug,
            0.0,
            0.0,
            self.point_on_circle(PI / 4.0),
            self.point_on_circle(PI + PI / 4.0),
        );
        draw_line(
            &ug,
            0.0,
            0.0,
            self.point_on_circle(-PI / 4.0),
            self.point_on_circle(PI - PI / 4.0),
        );
    }
}

impl Extremity for ExtremityCircleCross {}
