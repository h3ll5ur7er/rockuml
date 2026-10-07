//! A port on the border of its component: a small square with its name inside the component (PlantUML's
//! `EntityImagePort` and `AbstractEntityImageBorder`).

use crate::abel::{Entity, EntityId, EntityPosition};
use crate::color::{ColorType, HColor};
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::group::UGroup;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::component::creole_text;
use crate::style::{PName, SName, StyleSignature, ValueReading};
use crate::svek::{AbstractEntityImage, IEntityImage, LayoutContext, ShapeType};

use super::entity_group;

const SIDE: f64 = EntityPosition::RADIUS * 2.0;

pub(crate) struct EntityImagePort {
    base: AbstractEntityImage,
    /// The component the port is on.
    parent: Option<EntityId>,
    desc: Box<dyn TextBlock>,
    border_color: HColor,
    backcolor: HColor,
    group: UGroup,
}

impl EntityImagePort {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let skin = diagram.skin();
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            diagram.get_style_name(),
            SName::Port,
        ])
        .get_merged_style_with(&skin.current_style_builder(), entity.stereotype.as_ref());
        let desc = creole_text(
            entity.display.lines(),
            style.font_configuration(),
            HorizontalAlignment::Center,
            0.0,
            skin,
        );
        Self {
            base: AbstractEntityImage::new(entity, diagram),
            parent: entity.get_parent_container(diagram),
            desc,
            border_color: entity
                .colors
                .get(ColorType::Line)
                .cloned()
                .unwrap_or_else(|| style.value(PName::LineColor).as_color()),
            backcolor: entity
                .colors
                .get(ColorType::Back)
                .cloned()
                .unwrap_or_else(|| style.value(PName::BackGroundColor).as_color()),
            group: entity_group(entity, diagram, "entity", entity.get_location()),
        }
    }

    /// The square, with the name above it when `up_position`, else below.
    fn draw_at(&self, ug: &UGraphic, up_position: bool) {
        let dim_desc = self.desc.calculate_dimension(ug.string_bounder());
        let x = -(dim_desc.width - SIDE) / 2.0;
        let y = if up_position {
            -(SIDE + dim_desc.height)
        } else {
            SIDE
        };
        ug.start_group(&self.group);
        self.desc.draw_u(&ug.translated(x, y));
        ug.with_color(self.border_color.clone())
            .with_stroke(UStroke::with_thickness(1.5))
            .with_backcolor(self.backcolor.clone())
            .draw(&UShape::Rectangle(URectangle::new(SIDE, SIDE)));
        ug.close_group();
    }
}

impl TextBlock for EntityImagePort {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(SIDE, SIDE)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.draw_at(ug, false);
    }
}

impl IEntityImage for EntityImagePort {
    fn get_shape_type(&self) -> ShapeType {
        ShapeType::RectanglePort
    }

    fn is_hidden(&self) -> bool {
        self.base.is_hidden()
    }

    /// A port in the upper half of its component has its name above it.
    fn draw_u_in_layout(&self, ug: &UGraphic, layout: &LayoutContext<'_>) {
        let up_position = self
            .parent
            .and_then(|parent| layout.bibliotekon.get_cluster(parent))
            .is_some_and(|parent| {
                let center = parent.get_rectangle_area().get_point_center();
                layout.get_node(self.base.get_entity()).get_min_y() < center.y
            });
        self.draw_at(ug, up_position);
    }
}
