//! An initial state, `[*]` where a transition starts (PlantUML's `EntityImageCircleStart`).

use super::circle_start::CircleStart;
use super::entity_group;
use crate::abel::Entity;
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::group::UGroup;
use crate::klimt::ugraphic::UGraphic;
use crate::style::{SName, StyleSignature};
use crate::svek::{AbstractEntityImage, IEntityImage, ShapeType};

pub(crate) struct EntityImageCircleStart {
    base: AbstractEntityImage,
    group: UGroup,
    circle: CircleStart,
}

impl EntityImageCircleStart {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            diagram.get_style_name(),
            SName::Circle,
            SName::Start,
        ])
        .get_merged_style(&diagram.skin().current_style_builder());
        Self {
            base: AbstractEntityImage::new(entity, diagram),
            group: entity_group(entity, diagram, "start_entity", entity.get_location()),
            circle: CircleStart::new(&style, &entity.colors),
        }
    }
}

impl TextBlock for EntityImageCircleStart {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.circle.calculate_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.start_group(&self.group);
        self.circle.draw_u(ug);
        ug.close_group();
    }
}

impl IEntityImage for EntityImageCircleStart {
    fn get_shape_type(&self) -> ShapeType {
        ShapeType::Circle
    }

    fn is_hidden(&self) -> bool {
        self.base.is_hidden()
    }
}
