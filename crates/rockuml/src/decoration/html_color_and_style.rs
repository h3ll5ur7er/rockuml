//! One colour of an arrow with its line style and arrowhead colour (PlantUML's `HtmlColorAndStyle`).

use super::LinkStyle;
use crate::color::{HColor, NoSuchColor};
use crate::java;
use crate::skin::SkinParam;
use crate::style::{PName, SName, StyleSignature, ValueReading};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HtmlColorAndStyle {
    arrow_head_color: HColor,
    arrow_color: HColor,
    style: LinkStyle,
}

impl HtmlColorAndStyle {
    /// A plain line; its arrowhead in `arrow_head_color`, or else the line's colour.
    pub(crate) fn new(color: HColor, arrow_head_color: Option<HColor>) -> Self {
        Self::with_style(color, LinkStyle::NORMAL, arrow_head_color)
    }

    pub(crate) fn with_style(
        arrow_color: HColor,
        style: LinkStyle,
        arrow_head_color: Option<HColor>,
    ) -> Self {
        Self {
            arrow_head_color: arrow_head_color.unwrap_or_else(|| arrow_color.clone()),
            arrow_color,
            style,
        }
    }

    pub(crate) fn get_arrow_color(&self) -> &HColor {
        &self.arrow_color
    }

    pub(crate) fn get_arrow_head_color(&self) -> &HColor {
        &self.arrow_head_color
    }

    pub(crate) fn get_style(&self) -> LinkStyle {
        self.style
    }

    /// The style activity diagrams draw arrows with (`getDefaultStyleDefinitionArrow`).
    pub(crate) fn get_default_style_definition_arrow() -> StyleSignature {
        StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Arrow,
        ])
    }

    /// One colour of an arrow's specification, such as `#red,dashed`: each `,`-separated word is a line
    /// style or a colour, the last of each kind winning; what it does not name comes from the arrow style.
    pub(crate) fn build(skin_param: &SkinParam, definition: &str) -> Result<Self, NoSuchColor> {
        let style = skin_param.merged_style(&Self::get_default_style_definition_arrow());
        let mut arrow_color = style
            .as_ref()
            .and_then(|style| style.value(PName::LineColor))
            .as_color();
        let mut link_style = LinkStyle::NORMAL;
        for word in java::split(definition, ",") {
            let word_style = LinkStyle::from_string1(&word);
            if !word_style.is_normal() {
                link_style = word_style;
                continue;
            }
            arrow_color = HColor::parse(&word)
                .ok()
                .flatten()
                .ok_or_else(|| NoSuchColor(word.clone()))?;
        }
        Ok(Self::with_style(arrow_color, link_style, None))
    }
}
