//! A diamond where transitions branch, like a state's `<<choice>>` (PlantUML's `EntityImageBranch`).

use super::entity_group;
use crate::abel::Entity;
use crate::color::HColor;
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::group::UGroup;
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::style::{PName, SName, StyleSignature, ValueReading};
use crate::svek::IEntityImage;

const SIZE: f64 = 12.0;

pub(crate) struct EntityImageBranch {
    group: UGroup,
    border: HColor,
    back: HColor,
    stroke: UStroke,
}

impl EntityImageBranch {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Activity,
            SName::Diamond,
        ])
        .get_merged_style(&diagram.skin().current_style_builder());
        Self {
            group: entity_group(entity, diagram, "entity", None),
            border: style.value(PName::LineColor).as_color(),
            back: style.value(PName::BackGroundColor).as_color(),
            stroke: style.stroke(),
        }
    }
}

impl TextBlock for EntityImageBranch {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(SIZE * 2.0, SIZE * 2.0)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let diamond = UShape::Polygon(vec![
            (SIZE, 0.0),
            (SIZE * 2.0, SIZE),
            (SIZE, SIZE * 2.0),
            (0.0, SIZE),
            (SIZE, 0.0),
        ]);
        ug.start_group(&self.group);
        ug.with_color(self.border.clone())
            .with_backcolor(self.back.clone())
            .with_stroke(self.stroke)
            .draw(&diamond);
        ug.close_group();
    }
}

impl IEntityImage for EntityImageBranch {}
