//! The bar where transitions fork or join, like a state's `<<fork>>` (PlantUML's `EntityImageSynchroBar`).

use crate::abel::Entity;
use crate::color::HColor;
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::skin::Rankdir;
use crate::style::{PName, SName, StyleSignature, ValueReading};
use crate::svek::{AbstractEntityImage, IEntityImage, ShapeType};

pub(crate) struct EntityImageSynchroBar {
    base: AbstractEntityImage,
    rankdir: Rankdir,
    color: HColor,
}

impl EntityImageSynchroBar {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::ActivityBar,
        ])
        .get_merged_style_with(
            &diagram.skin().current_style_builder(),
            entity.stereotype.as_ref(),
        );
        Self {
            base: AbstractEntityImage::new(entity, diagram),
            rankdir: diagram.skin().get_rankdir(),
            color: style.value(PName::BackGroundColor).as_color(),
        }
    }
}

impl TextBlock for EntityImageSynchroBar {
    /// Across the direction the graph is laid out in.
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        match self.rankdir {
            Rankdir::LeftToRight => XDimension2D::new(8.0, 80.0),
            Rankdir::TopToBottom => XDimension2D::new(80.0, 8.0),
        }
    }

    fn draw_u(&self, ug: &UGraphic) {
        let dimension = self.calculate_dimension(ug.string_bounder());
        ug.with_color(HColor::NONE)
            .with_backcolor(self.color.clone())
            .draw(&UShape::Rectangle(URectangle::new(
                dimension.width,
                dimension.height,
            )));
    }
}

impl IEntityImage for EntityImageSynchroBar {
    fn get_shape_type(&self) -> ShapeType {
        ShapeType::Rectangle
    }

    fn is_hidden(&self) -> bool {
        self.base.is_hidden()
    }
}
