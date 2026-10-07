//! A socket's arc around the end of the link: `(`.

use std::f64::consts::{FRAC_PI_2, PI};

use super::{Extremity, ExtremityFactory};
use crate::klimt::UDrawable;
use crate::klimt::geom::XPoint2D;
use crate::klimt::shape::{UEllipse, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};

pub(crate) struct ExtremityFactoryParenthesis;

impl ExtremityFactory for ExtremityFactoryParenthesis {
    fn create_udrawable(&self, p0: XPoint2D, angle: f64) -> Box<dyn Extremity> {
        Box::new(ExtremityParenthesis {
            dest: p0,
            ortho: angle - FRAC_PI_2,
        })
    }
}

const RADIUS2: f64 = 9.0;
/// Half the arc's sweep, in degrees.
const ANG: f64 = 70.0;

struct ExtremityParenthesis {
    dest: XPoint2D,
    ortho: f64,
}

impl UDrawable for ExtremityParenthesis {
    fn draw_u(&self, ug: &UGraphic) {
        let deg = -self.ortho * 180.0 / PI + 90.0 - ANG;
        let arc1 = UEllipse::arc(2.0 * RADIUS2, 2.0 * RADIUS2, deg, 2.0 * ANG);
        ug.with_stroke(UStroke::with_thickness(1.5))
            .translated(self.dest.x - RADIUS2, self.dest.y - RADIUS2)
            .draw(&UShape::Ellipse(arc1));
    }
}

impl Extremity for ExtremityParenthesis {
    fn decoration_length(&self) -> f64 {
        10.0
    }
}
