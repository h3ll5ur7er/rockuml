//! The line between two swimlanes, with the room on either side of it (PlantUML's `LaneDivider`).

use crate::color::HColor;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XDimension2D};
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::skin::SkinParam;
use crate::style::{PName, SName, StyleSignature, ValueReading};

pub(crate) struct LaneDivider {
    /// The room left of the line.
    x1: f64,
    /// The room right of the line.
    x2: f64,
    height: f64,
    color: HColor,
    thickness: UStroke,
}

impl LaneDivider {
    pub(crate) fn new(skin_param: &SkinParam, x1: f64, x2: f64, height: f64) -> Self {
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Swimlane,
        ])
        .get_merged_style(&skin_param.current_style_builder());
        Self {
            x1,
            x2,
            height,
            color: style.value(PName::LineColor).as_color(),
            thickness: style.stroke(),
        }
    }

    pub(crate) fn get_width(&self) -> f64 {
        self.x1 + self.x2
    }

    pub(crate) fn get_x1(&self) -> f64 {
        self.x1
    }

    pub(crate) fn get_x2(&self) -> f64 {
        self.x2
    }
}

impl TextBlock for LaneDivider {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(self.x1 + self.x2, self.height)
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.draw(&UShape::Empty(XDimension2D::new(self.x1 + self.x2, 1.0)));
        ug.apply(UTranslate::new(self.x1, 0.0))
            .with_stroke(self.thickness)
            .with_color(self.color.clone())
            .draw(&UShape::Line {
                dx: 0.0,
                dy: self.height,
            });
    }
}
