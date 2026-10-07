//! An open arrowhead pointing at a circle.

use std::f64::consts::FRAC_PI_2;

use super::{Extremity, ExtremityFactory, change_back, manageround, rotated_polygon};
use crate::color::HColor;
use crate::klimt::UDrawable;
use crate::klimt::geom::XPoint2D;
use crate::klimt::shape::{UEllipse, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};

pub(crate) struct ExtremityFactoryArrowAndCircle {
    background_color: HColor,
}

impl ExtremityFactoryArrowAndCircle {
    pub(crate) fn new(background_color: HColor) -> Self {
        Self { background_color }
    }
}

impl ExtremityFactory for ExtremityFactoryArrowAndCircle {
    fn create_udrawable(&self, p0: XPoint2D, angle: f64) -> Box<dyn Extremity> {
        Box::new(ExtremityArrowAndCircle::new(
            p0,
            angle - FRAC_PI_2,
            self.background_color.clone(),
        ))
    }
}

const RADIUS: f64 = 5.0;

struct ExtremityArrowAndCircle {
    polygon: UShape,
    dest: XPoint2D,
    background_color: HColor,
}

impl ExtremityArrowAndCircle {
    fn new(p1: XPoint2D, angle: f64, background_color: HColor) -> Self {
        const X_WING: f64 = 9.0;
        const Y_APERTURE: f64 = 4.0;
        const X_CONTACT: f64 = 5.0;
        let angle = manageround(angle);
        let points = [
            (0.0, 0.0),
            (-X_WING, -Y_APERTURE),
            (-X_CONTACT, 0.0),
            (-X_WING, Y_APERTURE),
            (0.0, 0.0),
        ];
        let tip = XPoint2D::new(p1.x + RADIUS * angle.sin(), p1.y - RADIUS * angle.cos());
        Self {
            polygon: rotated_polygon(&points, angle + FRAC_PI_2, tip),
            dest: p1,
            background_color,
        }
    }
}

impl UDrawable for ExtremityArrowAndCircle {
    fn draw_u(&self, ug: &UGraphic) {
        change_back(ug).draw(&self.polygon);
        ug.with_stroke(UStroke::with_thickness(1.5))
            .with_backcolor(self.background_color.clone())
            .translated(self.dest.x - RADIUS, self.dest.y - RADIUS)
            .draw(&UShape::Ellipse(UEllipse::new(RADIUS * 2.0, RADIUS * 2.0)));
    }
}

impl Extremity for ExtremityArrowAndCircle {}
