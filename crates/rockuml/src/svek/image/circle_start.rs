//! The filled circle of an initial state (PlantUML's `CircleStart`).

use crate::color::{Colors, HColor};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{UEllipse, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::style::{PName, Style};

const SIZE: f64 = 20.0;

pub(crate) struct CircleStart {
    back_color: HColor,
    line_color: HColor,
    stroke: UStroke,
}

impl CircleStart {
    /// The element's own colours win over the style's.
    pub(crate) fn new(style: &Style, colors: &Colors) -> Self {
        Self {
            back_color: colors.get_color_of(style, PName::BackGroundColor),
            line_color: colors.get_color_of(style, PName::LineColor),
            stroke: style.stroke(),
        }
    }
}

impl TextBlock for CircleStart {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(SIZE, SIZE)
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.with_color(self.line_color.clone())
            .with_backcolor(self.back_color.clone())
            .with_stroke(self.stroke)
            .draw(&UShape::Ellipse(UEllipse::new(SIZE, SIZE)));
    }
}
