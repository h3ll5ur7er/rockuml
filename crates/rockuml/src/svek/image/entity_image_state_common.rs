//! What the boxes of states share: the name, the colours and the rounded outline (PlantUML's
//! `EntityImageStateCommon`).

use crate::abel::Entity;
use crate::color::{ColorType, Colors, HColor};
use crate::creole::{CreoleMode, SheetBlock2};
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::url::Url;
use crate::stereo::Stereotype;
use crate::style::{PName, SName, Style, StyleBuilder, StyleSignature, ValueReading};
use crate::svek::AbstractEntityImage;

/// `root, element, stateDiagram, state`, whatever the diagram.
pub(crate) fn state_signature() -> StyleSignature {
    StyleSignature::of(&[
        SName::Root,
        SName::Element,
        SName::StateDiagram,
        SName::State,
    ])
}

/// The style of a state's `part`, like its name or description, or of the whole state.
pub(crate) fn get_style_state(
    part: Option<SName>,
    stereotype: Option<&Stereotype>,
    builder: &StyleBuilder,
) -> Style {
    let signature = match part {
        Some(part) => state_signature().with_name(part),
        None => state_signature(),
    };
    signature.get_merged_style_with(builder, stereotype)
}

pub(crate) struct EntityImageStateCommon {
    pub base: AbstractEntityImage,
    pub name: SheetBlock2,
    pub url: Option<Url>,
    /// The colours the state sets for itself.
    pub colors: Colors,
    pub style_state: Style,
}

impl EntityImageStateCommon {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let builder = diagram.skin().current_style_builder();
        let stereotype = entity.stereotype.as_ref();
        let style_state = get_style_state(None, stereotype, &builder);
        let name_font = get_style_state(Some(SName::Name), stereotype, &builder)
            .font_configuration_with(&entity.colors);
        let name = entity.display.create0(
            name_font,
            style_state.horizontal_alignment().unwrap_or_default(),
            diagram.skin(),
            style_state.wrap_width(),
            CreoleMode::Full,
        );
        Self {
            base: AbstractEntityImage::new(entity, diagram),
            name,
            url: entity.url.clone(),
            colors: entity.colors.clone(),
            style_state,
        }
    }

    /// The rounded box around the state.
    pub(crate) fn get_shape(&self, dim_total: XDimension2D) -> UShape {
        let corner = self.style_state.value(PName::RoundCorner).as_double();
        UShape::Rectangle(URectangle::new(dim_total.width, dim_total.height).rounded(corner))
    }

    /// The border colour, and the state's own background or else `style`'s.
    pub(crate) fn apply_color(&self, ug: &UGraphic, style: &Style) -> UGraphic {
        let backcolor = self
            .colors
            .get(ColorType::Back)
            .cloned()
            .unwrap_or_else(|| style.value(PName::BackGroundColor).as_color());
        ug.with_color(self.get_border_color())
            .with_backcolor(backcolor)
    }

    pub(crate) fn get_border_color(&self) -> HColor {
        self.colors
            .get(ColorType::Line)
            .cloned()
            .unwrap_or_else(|| self.style_state.value(PName::LineColor).as_color())
    }
}
