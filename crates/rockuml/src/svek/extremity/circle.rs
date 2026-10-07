//! A circle beyond the end of the link: `0` hollow, `@` filled.

use std::f64::consts::FRAC_PI_2;

use super::{Extremity, ExtremityFactory, change_back};
use crate::color::HColor;
use crate::klimt::UDrawable;
use crate::klimt::geom::XPoint2D;
use crate::klimt::shape::{UEllipse, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};

pub(crate) struct ExtremityFactoryCircle {
    fill: bool,
    background_color: HColor,
}

impl ExtremityFactoryCircle {
    pub(crate) fn new(fill: bool, background_color: HColor) -> Self {
        Self {
            fill,
            background_color,
        }
    }
}

impl ExtremityFactory for ExtremityFactoryCircle {
    fn create_udrawable(&self, center: XPoint2D, angle: f64) -> Box<dyn Extremity> {
        Box::new(ExtremityCircle::new(
            center,
            self.fill,
            angle - FRAC_PI_2,
            self.background_color.clone(),
        ))
    }
}

const RADIUS: f64 = 6.0;

struct ExtremityCircle {
    dest: XPoint2D,
    fill: bool,
    background_color: HColor,
}

impl ExtremityCircle {
    fn new(center: XPoint2D, fill: bool, angle: f64, background_color: HColor) -> Self {
        Self {
            dest: XPoint2D::new(
                center.x - RADIUS * (angle + FRAC_PI_2).cos(),
                center.y - RADIUS * (angle + FRAC_PI_2).sin(),
            ),
            fill,
            background_color,
        }
    }
}

impl UDrawable for ExtremityCircle {
    fn draw_u(&self, ug: &UGraphic) {
        let ug = ug.with_stroke(UStroke::with_thickness(1.5));
        let ug = if self.fill {
            change_back(&ug)
        } else {
            ug.with_backcolor(self.background_color.clone())
        };
        ug.translated(self.dest.x - RADIUS, self.dest.y - RADIUS)
            .draw(&UShape::Ellipse(UEllipse::new(RADIUS * 2.0, RADIUS * 2.0)));
    }
}

impl Extremity for ExtremityCircle {
    fn decoration_length(&self) -> f64 {
        12.0
    }
}
