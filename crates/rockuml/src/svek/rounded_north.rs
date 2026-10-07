//! The top of a rounded box, painted in its own colour (PlantUML's `RoundedNorth`).

use crate::color::HColor;
use crate::klimt::shape::URectangle;
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
        let header = URectangle::new(self.width, self.height).half_rounded(self.rounded);
        ug.with_stroke(UStroke::SIMPLE)
            .with_color(self.back_color.clone())
            .with_backcolor(self.back_color.clone())
            .draw(&header);
    }
}
