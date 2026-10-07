//! What the boxes of states share: the name, the colours and the rounded outline (PlantUML's
//! `EntityImageStateCommon`).

use crate::abel::Entity;
use crate::color::{ColorType, Colors, HColor};
use crate::creole::{CreoleMode, Display, SheetBlock2};
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::TextBlock;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::url::Url;
use crate::skin::component::TextBlockEmpty;
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

/// The description lines of a composite state, drawn under its name (`Entity.getStateDescription`).
pub(crate) fn get_state_description(group: &Entity, diagram: &CucaDiagram) -> Box<dyn TextBlock> {
    let details = group.bodier.get_raw_body();
    if details.is_empty() {
        return Box::new(TextBlockEmpty::default());
    }
    let style = get_style_state(
        Some(SName::Description),
        group.stereotype.as_ref(),
        &diagram.skin().current_style_builder(),
    );
    let lines = details
        .iter()
        .flat_map(|line| Display::with_newlines(line).lines().to_vec());
    Box::new(Display::create(lines).create0(
        style.font_configuration(),
        style.horizontal_alignment().unwrap_or_default(),
        diagram.skin(),
        0.0,
        CreoleMode::Full,
    ))
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
