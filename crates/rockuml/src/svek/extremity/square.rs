//! A square centred on the end of the link: `#`.

use super::{Extremity, ExtremityFactory};
use crate::color::HColor;
use crate::klimt::UDrawable;
use crate::klimt::geom::XPoint2D;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};

pub(crate) struct ExtremityFactorySquare {
    background_color: HColor,
}

impl ExtremityFactorySquare {
    pub(crate) fn new(background_color: HColor) -> Self {
        Self { background_color }
    }
}

impl ExtremityFactory for ExtremityFactorySquare {
    fn create_udrawable(&self, p0: XPoint2D, _angle: f64) -> Box<dyn Extremity> {
        Box::new(ExtremitySquare {
            dest: p0,
            background_color: self.background_color.clone(),
        })
    }
}

const RADIUS: f64 = 5.0;

struct ExtremitySquare {
    dest: XPoint2D,
    background_color: HColor,
}

impl UDrawable for ExtremitySquare {
    fn draw_u(&self, ug: &UGraphic) {
        ug.with_stroke(UStroke::with_thickness(1.5))
            .with_backcolor(self.background_color.clone())
            .translated(self.dest.x - RADIUS, self.dest.y - RADIUS)
            .draw(&UShape::Rectangle(URectangle::new(
                RADIUS * 2.0,
                RADIUS * 2.0,
            )));
    }
}

impl Extremity for ExtremitySquare {
    fn decoration_length(&self) -> f64 {
        5.0
    }
}
