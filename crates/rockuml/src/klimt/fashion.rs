//! The colours and stroke a symbol is drawn with (PlantUML's `Fashion`).

use super::ugraphic::{UGraphic, UStroke};
use crate::color::HColor;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Fashion {
    pub back_color: HColor,
    pub fore_color: HColor,
    pub stroke: UStroke,
    pub round_corner: f64,
    pub diagonal_corner: f64,
}

impl Fashion {
    pub(crate) fn new(back_color: HColor, fore_color: HColor) -> Self {
        Self {
            back_color,
            fore_color,
            stroke: UStroke::SIMPLE,
            round_corner: 0.0,
            diagonal_corner: 0.0,
        }
    }

    #[must_use]
    pub(crate) fn with_stroke(&self, stroke: UStroke) -> Self {
        Self {
            stroke,
            ..self.clone()
        }
    }

    pub(crate) fn apply(&self, ug: &UGraphic) -> UGraphic {
        self.apply_colors(ug).with_stroke(self.stroke)
    }

    pub(crate) fn apply_colors(&self, ug: &UGraphic) -> UGraphic {
        ug.with_color(self.fore_color.clone())
            .with_backcolor(self.back_color.clone())
    }
}
