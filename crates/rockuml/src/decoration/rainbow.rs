//! The colours and styles of an arrow, one per parallel line it is drawn with, like `-[#red;#blue,dashed]->`
//! (PlantUML's `Rainbow` and `HtmlColorAndStyle`).

use super::LinkStyle;
use crate::color::{HColor, NoSuchColor};
use crate::java;
use crate::skin::SkinParam;
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HtmlColorAndStyle {
    arrow_head_color: HColor,
    arrow_color: HColor,
    style: LinkStyle,
}

impl HtmlColorAndStyle {
    /// Without a head colour, the head takes the line's.
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

    pub(crate) fn get_arrow_color(&self) -> &HColor {
        &self.arrow_color
    }

    pub(crate) fn get_arrow_head_color(&self) -> &HColor {
        &self.arrow_head_color
    }

    pub(crate) fn get_style(&self) -> LinkStyle {
        self.style
    }

    /// `#red,dashed`: comma-separated colours and line styles, the last of each kind winning, over the
    /// colour arrows get from the style.
    pub(crate) fn build(skin: &SkinParam, definition: &str) -> Result<Self, NoSuchColor> {
        let style = skin
            .merged_style(&StyleSignature::of(&[
                SName::Root,
                SName::Element,
                SName::ActivityDiagram,
                SName::Arrow,
            ]))
            .expect("the skin styles arrows");
        let mut arrow_color = style.value(PName::LineColor).as_color();
        let mut link_style = LinkStyle::NORMAL;
        for s in java::split(definition, ",") {
            let tmp_style = LinkStyle::from_string1(&s);
            if !tmp_style.is_normal() {
                link_style = tmp_style;
                continue;
            }
            arrow_color = HColor::parse(&s)
                .ok()
                .flatten()
                .ok_or_else(|| NoSuchColor(s.clone()))?;
        }
        Ok(Self::new(arrow_color, link_style, None))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Rainbow {
    colors: Vec<HtmlColorAndStyle>,
    color_arrow_separation_space: i32,
}

impl Rainbow {
    pub(crate) fn none() -> Self {
        Self {
            colors: Vec::new(),
            color_arrow_separation_space: 0,
        }
    }

    pub(crate) fn from_color(arrow_color: HColor, arrow_head_color: Option<HColor>) -> Self {
        Self::of(HtmlColorAndStyle::new(
            arrow_color,
            LinkStyle::NORMAL,
            arrow_head_color,
        ))
    }

    /// The line and head colours a style gives arrows.
    pub(crate) fn build_from_style(style: &Style) -> Self {
        let color = style.value(PName::LineColor).as_color();
        let head = style.value(PName::HeadColor);
        let color_head = if head.is_some() {
            head.as_color()
        } else {
            color.clone()
        };
        Self::from_color(color, Some(color_head))
    }

    #[must_use]
    pub(crate) fn with_default(self, default_color: Self) -> Self {
        if self.size() == 0 {
            default_color
        } else {
            self
        }
    }

    pub(crate) fn of(color: HtmlColorAndStyle) -> Self {
        Self {
            colors: vec![color],
            color_arrow_separation_space: 0,
        }
    }

    /// `;`-separated lines, each read by [`HtmlColorAndStyle::build`]; none without a definition.
    pub(crate) fn build(
        skin: &SkinParam,
        color_string: Option<&str>,
        color_arrow_separation_space: i32,
    ) -> Result<Self, NoSuchColor> {
        let Some(color_string) = color_string else {
            return Ok(Self::none());
        };
        let colors = java::split(color_string, ";")
            .iter()
            .map(|s| HtmlColorAndStyle::build(skin, s))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            colors,
            color_arrow_separation_space,
        })
    }

    pub(crate) fn is_invisible(&self) -> bool {
        self.colors.iter().any(|color| color.style.is_invisible())
    }

    pub(crate) fn get_colors(&self) -> &[HtmlColorAndStyle] {
        &self.colors
    }

    pub(crate) fn get_color(&self) -> &HColor {
        &self.colors[0].arrow_color
    }

    pub(crate) fn get_arrow_head_color(&self) -> &HColor {
        &self.colors[0].arrow_head_color
    }

    pub(crate) fn get_color_arrow_separation_space(&self) -> i32 {
        self.color_arrow_separation_space
    }

    pub(crate) fn size(&self) -> usize {
        self.colors.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skin() -> SkinParam {
        SkinParam::default()
    }

    fn color(name: &str) -> HColor {
        HColor::parse(name).unwrap().unwrap()
    }

    #[test]
    fn a_bracketed_arrow_style_gives_colour_and_line() {
        let rainbow = Rainbow::build(&skin(), Some("#red,dashed"), 0).unwrap();
        assert_eq!(rainbow.size(), 1);
        assert_eq!(rainbow.get_color(), &color("red"));
        assert_eq!(rainbow.get_arrow_head_color(), &color("red"));
        assert_eq!(rainbow.get_colors()[0].get_style(), LinkStyle::DASHED);
        assert!(!rainbow.is_invisible());
    }

    #[test]
    fn semicolons_draw_parallel_lines() {
        let rainbow = Rainbow::build(&skin(), Some("#red;#blue,hidden"), 3).unwrap();
        assert_eq!(rainbow.size(), 2);
        assert_eq!(rainbow.get_colors()[1].get_arrow_color(), &color("blue"));
        assert!(rainbow.is_invisible());
        assert_eq!(rainbow.get_color_arrow_separation_space(), 3);
    }

    #[test]
    fn a_style_alone_keeps_the_arrow_colour_of_the_skin() {
        let rainbow = Rainbow::build(&skin(), Some("bold"), 0).unwrap();
        assert_eq!(rainbow.get_color(), &color("#181818"));
        assert_eq!(rainbow.get_colors()[0].get_style(), LinkStyle::BOLD);
    }

    #[test]
    fn unknown_colours_are_errors() {
        assert_eq!(
            Rainbow::build(&skin(), Some("#nosuchcolor"), 0),
            Err(NoSuchColor("#nosuchcolor".to_owned()))
        );
    }

    #[test]
    fn no_definition_is_no_rainbow() {
        let none = Rainbow::build(&skin(), None, 0).unwrap();
        assert_eq!(none.size(), 0);
        let fallback = Rainbow::from_color(color("green"), None);
        assert_eq!(none.with_default(fallback.clone()), fallback);
    }
}
