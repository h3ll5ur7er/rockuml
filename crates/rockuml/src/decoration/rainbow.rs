//! The colours of an arrow, one line per colour, each with its own line style (PlantUML's `Rainbow` and
//! `HtmlColorAndStyle`).

use super::LinkStyle;
use crate::color::{HColor, NoSuchColor};
use crate::java;
use crate::skin::SkinParam;
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Rainbow {
    colors: Vec<HtmlColorAndStyle>,
    color_arrow_separation_space: i32,
}

impl Rainbow {
    pub(crate) const fn none() -> Self {
        Self {
            colors: Vec::new(),
            color_arrow_separation_space: 0,
        }
    }

    /// No colour makes no rainbow.
    pub(crate) fn from_color(
        arrow_color: Option<HColor>,
        arrow_head_color: Option<HColor>,
    ) -> Self {
        match arrow_color {
            Some(arrow_color) => Self::build(HtmlColorAndStyle::new(
                arrow_color,
                LinkStyle::NORMAL,
                arrow_head_color,
            )),
            None => Self::none(),
        }
    }

    /// The line colour a style gives arrows.
    pub(crate) fn build_from_style(style: &Style) -> Self {
        Self::from_color(Some(style.value(PName::LineColor).as_color()), None)
    }

    pub(crate) fn build(color: HtmlColorAndStyle) -> Self {
        Self {
            colors: vec![color],
            color_arrow_separation_space: 0,
        }
    }

    /// The colours of an arrow written like `#red;#blue,dashed`: one per `;`, each painted over the style's.
    pub(crate) fn build_from_definition(
        skin: &SkinParam,
        color_string: &str,
        color_arrow_separation_space: i32,
    ) -> Result<Self, NoSuchColor> {
        let colors = java::split(color_string, ";")
            .iter()
            .map(|definition| HtmlColorAndStyle::build(skin, definition))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            colors,
            color_arrow_separation_space,
        })
    }

    /// The first colour.
    ///
    /// # Panics
    ///
    /// On a rainbow without colours, as PlantUML fails.
    pub(crate) fn get_color(&self) -> &HColor {
        &self.colors[0].arrow_color
    }

    pub(crate) fn size(&self) -> usize {
        self.colors.len()
    }
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "activity arrows are not drawn yet")
)]
impl Rainbow {
    pub(crate) fn get_colors(&self) -> &[HtmlColorAndStyle] {
        &self.colors
    }

    pub(crate) fn get_color_arrow_separation_space(&self) -> i32 {
        self.color_arrow_separation_space
    }
}

/// One colour of an arrow, with the colour of its head and the style of its line.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HtmlColorAndStyle {
    arrow_head_color: HColor,
    arrow_color: HColor,
    style: LinkStyle,
}

impl HtmlColorAndStyle {
    /// The head takes the arrow's colour unless it has its own.
    pub(crate) fn new(
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

    /// A definition like `#red,dashed`: a line style or a colour per `,`, over the activity arrows' style
    /// read from `skin` now.
    fn build(skin: &SkinParam, definition: &str) -> Result<Self, NoSuchColor> {
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Arrow,
        ])
        .get_merged_style(&skin.current_style_builder());
        let mut arrow_color = style.value(PName::LineColor).as_color();
        let mut link_style = LinkStyle::NORMAL;
        for part in java::split(definition, ",") {
            let part_style = LinkStyle::from_string1(&part);
            if !part_style.is_normal() {
                link_style = part_style;
                continue;
            }
            arrow_color = HColor::parse(&part)
                .ok()
                .flatten()
                .ok_or(NoSuchColor(part))?;
        }
        Ok(Self::new(arrow_color, link_style, None))
    }
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "activity arrows are not drawn yet")
)]
impl HtmlColorAndStyle {
    pub(crate) fn get_arrow_color(&self) -> &HColor {
        &self.arrow_color
    }

    pub(crate) fn get_arrow_head_color(&self) -> &HColor {
        &self.arrow_head_color
    }

    pub(crate) fn get_style(&self) -> LinkStyle {
        self.style
    }
}
