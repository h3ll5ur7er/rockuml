//! The circled plus of nesting: `+`.

use std::f64::consts::{FRAC_PI_2, PI};

use super::{Extremity, ExtremityFactory, draw_line};
use crate::color::HColor;
use crate::klimt::UDrawable;
use crate::klimt::geom::XPoint2D;
use crate::klimt::shape::{UEllipse, UShape};
use crate::klimt::ugraphic::UGraphic;

pub(crate) struct ExtremityFactoryPlus {
    background_color: HColor,
}

impl ExtremityFactoryPlus {
    pub(crate) fn new(background_color: HColor) -> Self {
        Self { background_color }
    }
}

impl ExtremityFactory for ExtremityFactoryPlus {
    fn create_udrawable(&self, center: XPoint2D, angle: f64) -> Box<dyn Extremity> {
        Box::new(ExtremityPlus::create(
            center,
            angle - FRAC_PI_2,
            self.background_color.clone(),
        ))
    }
}

const RADIUS: f64 = 8.0;

struct ExtremityPlus {
    px: f64,
    py: f64,
    angle: f64,
    background_color: HColor,
}

impl ExtremityPlus {
    fn create(p1: XPoint2D, angle: f64, background_color: HColor) -> Self {
        Self {
            px: p1.x - RADIUS + RADIUS * angle.sin(),
            py: p1.y - RADIUS - RADIUS * angle.cos(),
            angle,
            background_color,
        }
    }

    fn point_on_circle(&self, angle: f64) -> XPoint2D {
        XPoint2D::new(
            self.px + RADIUS + RADIUS * angle.cos(),
            self.py + RADIUS + RADIUS * angle.sin(),
        )
    }
}

impl UDrawable for ExtremityPlus {
    fn draw_u(&self, ug: &UGraphic) {
        ug.with_backcolor(self.background_color.clone())
            .translated(self.px, self.py)
            .draw(&UShape::Ellipse(UEllipse::new(2.0 * RADIUS, 2.0 * RADIUS)));
        let angle = self.angle;
        draw_line(
            ug,
            0.0,
            0.0,
            self.point_on_circle(angle - PI / 2.0),
            self.point_on_circle(angle + PI / 2.0),
        );
        draw_line(
            ug,
            0.0,
            0.0,
            self.point_on_circle(angle),
            self.point_on_circle(angle + PI),
        );
    }
}

impl Extremity for ExtremityPlus {
    fn decoration_length(&self) -> f64 {
        16.0
    }
}
