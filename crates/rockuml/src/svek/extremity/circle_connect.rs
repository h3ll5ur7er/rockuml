//! A circle in a socket's arc: `0)`.

use std::f64::consts::{FRAC_PI_2, PI};

use super::{Extremity, ExtremityFactory};
use crate::color::HColor;
use crate::klimt::UDrawable;
use crate::klimt::geom::XPoint2D;
use crate::klimt::shape::{UEllipse, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};

pub(crate) struct ExtremityFactoryCircleConnect {
    background_color: HColor,
}

impl ExtremityFactoryCircleConnect {
    pub(crate) fn new(background_color: HColor) -> Self {
        Self { background_color }
    }
}

impl ExtremityFactory for ExtremityFactoryCircleConnect {
    fn create_udrawable(&self, p0: XPoint2D, angle: f64) -> Box<dyn Extremity> {
        Box::new(ExtremityCircleConnect {
            dest: p0,
            ortho: angle - FRAC_PI_2,
            background_color: self.background_color.clone(),
        })
    }
}

const RADIUS: f64 = 6.0;
const RADIUS2: f64 = 10.0;

struct ExtremityCircleConnect {
    dest: XPoint2D,
    ortho: f64,
    background_color: HColor,
}

impl UDrawable for ExtremityCircleConnect {
    fn draw_u(&self, ug: &UGraphic) {
        let ug = ug
            .with_stroke(UStroke::with_thickness(1.5))
            .with_backcolor(self.background_color.clone());
        ug.translated(self.dest.x - RADIUS, self.dest.y - RADIUS)
            .draw(&UShape::Ellipse(UEllipse::new(RADIUS * 2.0, RADIUS * 2.0)));

        let deg = -self.ortho * 180.0 / PI + 90.0 - 45.0;
        let arc1 = UEllipse::arc(2.0 * RADIUS2, 2.0 * RADIUS2, deg, 90.0);
        ug.translated(self.dest.x - RADIUS2, self.dest.y - RADIUS2)
            .draw(&UShape::Ellipse(arc1));
    }
}

impl Extremity for ExtremityCircleConnect {
    fn decoration_length(&self) -> f64 {
        10.0
    }
}
