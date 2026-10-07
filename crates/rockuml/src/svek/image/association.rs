//! The diamond of an n-ary association, and the point an association class hangs from (PlantUML's
//! `EntityImageAssociation` and `EntityImageAssociationPoint`).

use crate::abel::Entity;
use crate::color::HColor;
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{UEllipse, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::style::{PName, SName, StyleSignature, ValueReading};
use crate::svek::{AbstractEntityImage, IEntityImage};

pub(crate) struct EntityImageAssociation {
    image: AbstractEntityImage,
    border_color: HColor,
    background_color: HColor,
    stroke: UStroke,
}

impl EntityImageAssociation {
    const SIZE: f64 = 12.0;

    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            diagram.get_style_name(),
            SName::Diamond,
        ])
        .get_merged_style(&diagram.skin().current_style_builder());
        Self {
            image: AbstractEntityImage::new(entity, diagram),
            border_color: style.value(PName::LineColor).as_color(),
            background_color: style.value(PName::BackGroundColor).as_color(),
            stroke: style.stroke(),
        }
    }
}

impl TextBlock for EntityImageAssociation {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(Self::SIZE * 2.0, Self::SIZE * 2.0)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let size = Self::SIZE;
        let diamond = UShape::Polygon(vec![
            (size, 0.0),
            (size * 2.0, size),
            (size, size * 2.0),
            (0.0, size),
            (size, 0.0),
        ]);
        ug.with_color(self.border_color.clone())
            .with_backcolor(self.background_color.clone())
            .with_stroke(self.stroke)
            .draw(&diamond);
    }

    fn backcolor(&self) -> Option<HColor> {
        Some(self.image.get_backcolor())
    }
}

impl IEntityImage for EntityImageAssociation {}

pub(crate) struct EntityImageAssociationPoint {
    image: AbstractEntityImage,
    color: HColor,
}

impl EntityImageAssociationPoint {
    const SIZE: f64 = 4.0;

    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ClassDiagram,
            SName::Arrow,
        ])
        .get_merged_style_with(
            &diagram.skin().current_style_builder(),
            entity.stereotype.as_ref(),
        );
        Self {
            image: AbstractEntityImage::new(entity, diagram),
            color: style.value(PName::LineColor).as_color(),
        }
    }
}

impl TextBlock for EntityImageAssociationPoint {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(Self::SIZE, Self::SIZE)
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.with_color(self.color.clone())
            .with_backcolor(self.color.clone())
            .draw(&UShape::Ellipse(UEllipse::new(Self::SIZE, Self::SIZE)));
    }

    fn backcolor(&self) -> Option<HColor> {
        Some(self.image.get_backcolor())
    }
}

impl IEntityImage for EntityImageAssociationPoint {}
