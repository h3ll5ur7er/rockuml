//! A lollipop interface: a small circle, or half of one, with its name below (PlantUML's
//! `EntityImageLollipopInterface`).

use super::entity_group;
use crate::abel::{Entity, LeafType};
use crate::color::HColor;
use crate::creole::CreoleMode;
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::group::UGroup;
use crate::klimt::shape::{UEllipse, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::url::Url;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::style::{PName, SName, StyleSignature, ValueReading};
use crate::svek::{AbstractEntityImage, IEntityImage};

const SIZE: f64 = 10.0;

pub(crate) struct EntityImageLollipopInterface {
    image: AbstractEntityImage,
    desc: Box<dyn TextBlock>,
    url: Option<Url>,
    group: UGroup,
    half: bool,
    background_color: HColor,
    border_color: HColor,
}

impl EntityImageLollipopInterface {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let skin = diagram.skin();
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            diagram.get_style_name(),
            SName::Circle,
        ])
        .get_merged_style_with(&skin.current_style_builder(), entity.stereotype.as_ref());
        let desc = entity.display.create0(
            &style.font_configuration(),
            HorizontalAlignment::Center,
            skin,
            0.0,
            CreoleMode::Full,
        );
        Self {
            image: AbstractEntityImage::new(entity, diagram),
            desc: Box::new(desc),
            url: entity.url.clone(),
            group: entity_group(entity, diagram, "entity", entity.get_location()),
            half: entity.get_leaf_type() == Some(LeafType::LollipopHalf),
            background_color: style.value(PName::BackGroundColor).as_color(),
            border_color: style.value(PName::LineColor).as_color(),
        }
    }
}

impl TextBlock for EntityImageLollipopInterface {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(SIZE, SIZE)
    }

    fn draw_u(&self, ug: &UGraphic) {
        // Graphviz would turn a half circle towards its link; Smetana leaves it facing up.
        let circle = if self.half {
            UEllipse::arc(SIZE, SIZE, -90.0, 180.0)
        } else {
            UEllipse::new(SIZE, SIZE)
        };
        let ug = ug
            .with_backcolor(self.background_color.clone())
            .with_color(self.border_color.clone());
        if let Some(url) = &self.url {
            ug.start_url(url);
        }
        ug.start_group(&self.group);
        ug.with_stroke(UStroke::with_thickness(1.5))
            .draw(&UShape::Ellipse(circle));
        ug.close_group();
        let width_desc = self.desc.calculate_dimension(ug.string_bounder()).width;
        self.desc
            .draw_u(&ug.translated(SIZE / 2.0 - width_desc / 2.0, SIZE));
        if self.url.is_some() {
            ug.close_url();
        }
    }

    fn backcolor(&self) -> Option<HColor> {
        Some(self.image.get_backcolor())
    }
}

impl IEntityImage for EntityImageLollipopInterface {}
