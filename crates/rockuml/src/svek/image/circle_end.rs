//! The ringed circle of a final state (PlantUML's `CircleEnd`).

use crate::color::{Colors, HColor};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{UEllipse, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::style::{PName, Style};

const SIZE: f64 = 22.0;
/// How far the inner disc keeps from the ring.
const DELTA: f64 = 5.0;

pub(crate) struct CircleEnd {
    back_color: HColor,
    line_color: HColor,
    stroke: UStroke,
}

impl CircleEnd {
    /// The element's own colours win over the style's.
    pub(crate) fn new(style: &Style, colors: &Colors) -> Self {
        Self {
            back_color: colors.get_color_of(style, PName::BackGroundColor),
            line_color: colors.get_color_of(style, PName::LineColor),
            stroke: style.stroke(),
        }
    }
}

impl TextBlock for CircleEnd {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(SIZE, SIZE)
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.with_backcolor(HColor::NONE)
            .with_color(self.line_color.clone())
            .with_stroke(self.stroke)
            .draw(&UShape::Ellipse(UEllipse::new(SIZE, SIZE)));
        // Antialiasing would blur a disc into a ring of the same colour, so PlantUML outlines it with the
        // colour halfway between the two.
        let outline = if self.line_color == self.back_color {
            HColor::Middle(self.line_color.as_xcolor(), self.back_color.as_xcolor())
        } else {
            self.line_color.clone()
        };
        ug.with_backcolor(self.back_color.clone())
            .with_color(outline)
            .translated(DELTA, DELTA)
            .draw(&UShape::Ellipse(UEllipse::new(
                SIZE - DELTA * 2.0,
                SIZE - DELTA * 2.0,
            )));
    }
}
