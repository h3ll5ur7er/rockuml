//! The colours of an arrow (PlantUML's `Rainbow`), as far as Smetana draws them: the line colour of a style.

use crate::color::HColor;
use crate::style::{PName, Style, ValueReading};

pub(crate) struct Rainbow {
    color: HColor,
}

impl Rainbow {
    /// The line colour a style gives arrows.
    pub(crate) fn build_from_style(style: &Style) -> Self {
        Self {
            color: style.value(PName::LineColor).as_color(),
        }
    }

    pub(crate) fn get_color(&self) -> &HColor {
        &self.color
    }
}
