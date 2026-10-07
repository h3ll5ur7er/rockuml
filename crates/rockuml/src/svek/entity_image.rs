//! The drawing of one entity as a node of the layout (PlantUML's `IEntityImage` and `AbstractEntityImage`).

#![cfg_attr(test, allow(dead_code, reason = "drawn by the Smetana bridge"))]

use super::{Margins, ShapeType};
use crate::abel::{Entity, EntityId};
use crate::color::HColor;
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
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
