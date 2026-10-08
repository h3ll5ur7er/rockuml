//! The cross of a non-navigable end: `x`.

use std::f64::consts::{FRAC_PI_2, PI};

use super::{Extremity, ExtremityFactory, manageround};
use crate::klimt::UDrawable;
use crate::klimt::affine::XAffineTransform;
use crate::klimt::geom::XPoint2D;
use crate::klimt::shape::{USegment, UShape};
use crate::klimt::ugraphic::UGraphic;

pub(crate) struct ExtremityFactoryNotNavigable;

impl ExtremityFactory for ExtremityFactoryNotNavigable {
    fn create_udrawable(&self, p0: XPoint2D, angle: f64) -> Box<dyn Extremity> {
        Box::new(ExtremityNotNavigable::new(p0, angle - FRAC_PI_2))
    }
}

struct ExtremityNotNavigable {
    path: UShape,
}

impl ExtremityNotNavigable {
    fn new(p1: XPoint2D, angle: f64) -> Self {
        const SIZE: f64 = 4.0;
        const MOVE: f64 = 5.0;
        let rotate = XAffineTransform::rotate_instance(manageround(angle) + PI);
        let segment = |kind: fn(f64, f64) -> USegment, x: f64, y: f64| {
            let (x, y) = rotate.transform((x, y + MOVE));
            // UPath.translate leaves a path moved by nothing alone, negative zeros included.
            if p1.x == 0.0 && p1.y == 0.0 {
                kind(x, y)
            } else {
                kind(x + p1.x, y + p1.y)
            }
        };
        Self {
            path: UShape::path(vec![
                segment(USegment::MoveTo, -SIZE, 0.0),
                segment(USegment::LineTo, SIZE, 2.0 * SIZE),
                segment(USegment::MoveTo, SIZE, 0.0),
                segment(USegment::LineTo, -SIZE, 2.0 * SIZE),
            ]),
        }
    }
}

impl UDrawable for ExtremityNotNavigable {
    fn draw_u(&self, ug: &UGraphic) {
        ug.draw(&self.path);
    }
}

impl Extremity for ExtremityNotNavigable {}
