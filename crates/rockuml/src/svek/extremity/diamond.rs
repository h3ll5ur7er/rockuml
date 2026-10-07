//! The diamond of aggregation (`o`, hollow) and composition (`*`, filled).

use std::f64::consts::FRAC_PI_2;

use super::{Extremity, ExtremityFactory, change_back, manageround, rotated_polygon};
use crate::color::HColor;
use crate::klimt::UDrawable;
use crate::klimt::geom::XPoint2D;
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::UGraphic;

pub(crate) struct ExtremityFactoryDiamond {
    fill: bool,
}

impl ExtremityFactoryDiamond {
    pub(crate) fn new(fill: bool) -> Self {
        Self { fill }
    }
}

impl ExtremityFactory for ExtremityFactoryDiamond {
    fn create_udrawable(&self, p0: XPoint2D, angle: f64) -> Box<dyn Extremity> {
        Box::new(ExtremityDiamond::new(p0, angle - FRAC_PI_2, self.fill))
    }
}

struct ExtremityDiamond {
    polygon: UShape,
    fill: bool,
}

impl ExtremityDiamond {
    fn new(p1: XPoint2D, angle: f64, fill: bool) -> Self {
        const X_WING: f64 = 6.0;
        const Y_APERTURE: f64 = 4.0;
        let points = [
            (0.0, 0.0),
            (-X_WING, -Y_APERTURE),
            (-X_WING * 2.0, 0.0),
            (-X_WING, Y_APERTURE),
            (0.0, 0.0),
        ];
        Self {
            polygon: rotated_polygon(&points, manageround(angle) + FRAC_PI_2, p1),
            fill,
        }
    }
}

impl UDrawable for ExtremityDiamond {
    fn draw_u(&self, ug: &UGraphic) {
        let ug = if self.fill {
            change_back(ug)
        } else {
            ug.with_backcolor(HColor::NONE)
        };
        ug.draw(&self.polygon);
    }
}

impl Extremity for ExtremityDiamond {
    fn decoration_length(&self) -> f64 {
        12.0
    }
}
