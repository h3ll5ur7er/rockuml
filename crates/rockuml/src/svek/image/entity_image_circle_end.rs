//! A final state, `[*]` where a transition ends (PlantUML's `EntityImageCircleEnd`).

use super::circle_end::CircleEnd;
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

const SIZE: f64 = 22.0;

pub(crate) struct EntityImageCircleEnd {
    base: AbstractEntityImage,
    group: UGroup,
    circle: CircleEnd,
}

impl EntityImageCircleEnd {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            diagram.get_style_name(),
            SName::Circle,
            SName::End,
        ])
        .get_merged_style(&diagram.skin().current_style_builder());
        Self {
            base: AbstractEntityImage::new(entity, diagram),
            group: entity_group(entity, diagram, "end_entity", entity.get_location()),
            circle: CircleEnd::new(&style, &entity.colors),
        }
    }
}

impl TextBlock for EntityImageCircleEnd {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(SIZE, SIZE)
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.start_group(&self.group);
        self.circle.draw_u(ug);
        ug.close_group();
    }
}

impl IEntityImage for EntityImageCircleEnd {
    fn get_shape_type(&self) -> ShapeType {
        ShapeType::Circle
    }

    fn is_hidden(&self) -> bool {
        self.base.is_hidden()
    }
}
