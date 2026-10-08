//! The bottom of a rounded box, painted in its own colour (PlantUML's `RoundedSouth`).

use crate::color::HColor;
use crate::klimt::shape::{URectangle, USegment, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};

pub(crate) struct RoundedSouth {
    pub width: f64,
    pub height: f64,
    pub back_color: HColor,
    pub rounded: f64,
}

impl RoundedSouth {
    pub(crate) fn draw_u(&self, ug: &UGraphic) {
        if self.back_color.is_transparent() {
            return;
        }
        let (width, height, r) = (self.width, self.height, self.rounded / 2.0);
        let footer = if self.rounded == 0.0 {
            UShape::Rectangle(URectangle::new(width, height))
        } else {
            UShape::path(vec![
                USegment::MoveTo(0.0, 0.0),
                USegment::LineTo(width, 0.0),
                USegment::LineTo(width, height - r),
                USegment::arc_to((width - r, height), r, true),
                USegment::LineTo(r, height),
                USegment::arc_to((0.0, height - r), r, true),
                USegment::LineTo(0.0, 0.0),
            ])
        };
        ug.with_stroke(UStroke::SIMPLE)
            .with_color(self.back_color.clone())
            .with_backcolor(self.back_color.clone())
            .draw(&footer);
    }
}
