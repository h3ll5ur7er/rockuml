//! A state on the border of its composite state, like an entry or exit point or a pin, with its name above
//! or below (PlantUML's `EntityImageStateBorder` and `AbstractEntityImageBorder`).

use crate::abel::{Entity, EntityId, EntityPosition};
use crate::color::{ColorType, HColor};
use crate::creole::{CreoleMode, SheetBlock2};
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::Rankdir;
use crate::style::{PName, ValueReading};
use crate::svek::{AbstractEntityImage, IEntityImage, LayoutContext, ShapeType};

use super::entity_image_state_common::get_style_state;

pub(crate) struct EntityImageStateBorder {
    base: AbstractEntityImage,
    entity_position: EntityPosition,
    rankdir: Rankdir,
    desc: SheetBlock2,
    border_color: HColor,
    backcolor: HColor,
    /// The composite state whose border the state sits on.
    parent: Option<EntityId>,
}

impl EntityImageStateBorder {
    /// # Panics
    ///
    /// For a state inside its composite state rather than on its border.
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let entity_position = entity.get_entity_position();
        assert!(
            !entity_position.is_normal(),
            "only states on a border are drawn so"
        );
        let style = get_style_state(
            None,
            entity.stereotype.as_ref(),
            &diagram.skin().current_style_builder(),
        );
        let desc = entity.display.create0(
            &style.font_configuration(),
            HorizontalAlignment::Center,
            diagram.skin(),
            0.0,
            CreoleMode::Full,
        );
        let mut backcolor = entity
            .colors
            .get(ColorType::Back)
            .cloned()
            .unwrap_or_else(|| style.value(PName::BackGroundColor).as_color());
        if backcolor.is_transparent() {
            backcolor = diagram.skin().get_background_color();
        }
        Self {
            base: AbstractEntityImage::new(entity, diagram),
            entity_position,
            rankdir: diagram.skin().get_rankdir(),
            desc,
            border_color: style.value(PName::LineColor).as_color(),
            backcolor,
            parent: entity.get_parent_container(diagram),
        }
    }
}

impl TextBlock for EntityImageStateBorder {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        self.entity_position.get_dimension(self.rankdir)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.draw(ug, false);
    }
}

impl EntityImageStateBorder {
    /// Whether the node sits in the upper half of its composite state, so that its name goes above it.
    fn up_position(&self, layout: &LayoutContext<'_>) -> bool {
        let Some(cluster) = self
            .parent
            .and_then(|parent| layout.bibliotekon.get_cluster(parent))
        else {
            return false;
        };
        let node = layout.get_node(self.base.get_entity());
        node.get_min_y() < cluster.get_rectangle_area().get_point_center().y
    }

    fn draw(&self, ug: &UGraphic, up_position: bool) {
        let dim_desc = self.desc.calculate_dimension(ug.string_bounder());
        let x = -(dim_desc.width - 2.0 * EntityPosition::RADIUS) / 2.0;
        let y = if up_position {
            -(2.0 * EntityPosition::RADIUS + dim_desc.height)
        } else {
            2.0 * EntityPosition::RADIUS
        };
        self.desc.draw_u(&ug.translated(x, y));
        let ug = ug
            .with_stroke(UStroke::with_thickness(1.5))
            .with_color(self.border_color.clone())
            .with_backcolor(self.backcolor.clone());
        self.entity_position.draw_symbol(&ug, self.rankdir);
    }
}

impl IEntityImage for EntityImageStateBorder {
    fn get_shape_type(&self) -> ShapeType {
        self.entity_position.get_shape_type()
    }

    fn is_hidden(&self) -> bool {
        self.base.is_hidden()
    }

    fn draw_u_in_layout(&self, ug: &UGraphic, layout: &LayoutContext<'_>) {
        self.draw(ug, self.up_position(layout));
    }
}
