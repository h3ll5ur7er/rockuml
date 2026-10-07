//! The drawing of one entity as a node of the layout (PlantUML's `IEntityImage` and `AbstractEntityImage`).

#![allow(
    dead_code,
    reason = "the images of the class, description, state and note families read the rest"
)]

use super::{Bibliotekon, Margins, ShapeType, SvekNode};
use crate::abel::{Entity, EntityId, LinkId};
use crate::color::HColor;
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XRectangle2D;
use crate::klimt::ugraphic::UGraphic;
use crate::sdot::SmetanaEdge;
use crate::stereo::Stereotype;
use crate::style::SName;

pub(crate) trait IEntityImage: TextBlock {
    fn get_shape_type(&self) -> ShapeType;

    /// Room links keep clear of around the drawing; none unless the image says so, as in
    /// `AbstractEntityImage`.
    fn get_shield(&self, _string_bounder: &dyn StringBounder) -> Margins {
        Margins::NONE
    }

    /// How far the drawing spills over the node's width on each side.
    fn get_overscan_x(&self, _string_bounder: &dyn StringBounder) -> f64 {
        0.0
    }

    fn is_hidden(&self) -> bool;

    /// Where the image draws the member of its entity that best matches `member`, which tips point at
    /// (`getBestMatch`, then `getInnerPosition`); images without members have none.
    fn get_inner_position(
        &self,
        _member: &str,
        _string_bounder: &dyn StringBounder,
    ) -> Option<XRectangle2D> {
        None
    }

    /// `EntityImageNote.setOpaleLink`: a note whose single link goes to `other` draws that link as part of
    /// its outline, which the layout then leaves out. The layout asks notes only.
    fn set_opale_link(&mut self, _link: LinkId, _other: EntityId) {
        unreachable!("only notes take their link into their outline")
    }

    /// Draws the image where the layout put it. Images that depend on the rest of the layout, like notes
    /// drawn around their link, find it in `layout`.
    fn draw_u_in_layout(&self, ug: &UGraphic, _layout: &LayoutContext<'_>) {
        self.draw_u(ug);
    }
}

/// What a laid out image can see of the rest of its layout while it is drawn.
pub(crate) struct LayoutContext<'a> {
    pub diagram: &'a CucaDiagram,
    pub bibliotekon: &'a Bibliotekon,
    /// The edges drawn, by link, in the order of the diagram's links.
    pub smetana_pathes: &'a [(LinkId, SmetanaEdge)],
}

impl LayoutContext<'_> {
    /// # Panics
    ///
    /// If the layout has no node for `leaf`.
    pub(crate) fn get_node(&self, leaf: EntityId) -> &SvekNode {
        self.bibliotekon
            .get_node(leaf)
            .expect("the layout has a node for every leaf it draws")
    }

    pub(crate) fn get_smetana_edge(&self, link: LinkId) -> Option<&SmetanaEdge> {
        self.smetana_pathes
            .iter()
            .find(|(known, _)| *known == link)
            .map(|(_, edge)| edge)
    }
}

/// What every entity's image knows of its entity, read when the image is made: images are made while the
/// diagram is drawn, and outlive no change that would alter these.
pub(crate) struct AbstractEntityImage {
    entity: EntityId,
    hidden: bool,
    backcolor: HColor,
    stereo: Option<Stereotype>,
    style_name: SName,
}

impl AbstractEntityImage {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        Self {
            entity: entity.id(),
            hidden: entity.is_hidden(diagram),
            backcolor: diagram.skin().get_background_color(),
            stereo: entity.stereotype.clone(),
            style_name: diagram.get_style_name(),
        }
    }

    pub(crate) fn is_hidden(&self) -> bool {
        self.hidden
    }

    pub(crate) fn get_entity(&self) -> EntityId {
        self.entity
    }

    /// The diagram's background, which images give `TextBlock::backcolor`.
    pub(crate) fn get_backcolor(&self) -> HColor {
        self.backcolor.clone()
    }

    pub(crate) fn get_stereo(&self) -> Option<&Stereotype> {
        self.stereo.as_ref()
    }

    pub(crate) fn get_style_name(&self) -> SName {
        self.style_name
    }
}
