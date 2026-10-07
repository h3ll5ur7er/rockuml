//! The top of a rounded box, painted in its own colour (PlantUML's `RoundedNorth`).

use crate::color::HColor;
use crate::klimt::shape::{URectangle, USegment, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};

pub(crate) struct RoundedNorth {
    pub width: f64,
    pub height: f64,
    pub back_color: HColor,
    pub rounded: f64,
}

impl RoundedNorth {
    pub(crate) fn draw_u(&self, ug: &UGraphic) {
        if self.back_color.is_transparent() {
            return;
        }
        let (width, height, r) = (self.width, self.height, self.rounded / 2.0);
        let header = if self.rounded == 0.0 {
            UShape::Rectangle(URectangle::new(width, height))
        } else {
            UShape::Path(vec![
                USegment::MoveTo(r, 0.0),
                USegment::LineTo(width - r, 0.0),
                arc_to(r, width, r),
                USegment::LineTo(width, height),
                USegment::LineTo(0.0, height),
                USegment::LineTo(0.0, r),
                arc_to(r, r, 0.0),
            ])
        };
        ug.with_stroke(UStroke::SIMPLE)
            .with_color(self.back_color.clone())
            .with_backcolor(self.back_color.clone())
            .draw(&header);
    }
}

/// A quarter circle of radius `r` to `(x, y)`, turning clockwise.
pub(crate) fn arc_to(r: f64, x: f64, y: f64) -> USegment {
    USegment::ArcTo {
        radius: (r, r),
        x_axis_rotation: 0.0,
        large_arc: false,
        sweep: true,
        end: (x, y),
    }
}
