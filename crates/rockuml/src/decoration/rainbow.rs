//! The colours of an arrow (PlantUML's `Rainbow`): none, one, or several drawn side by side, each with its
//! own line style and arrowhead colour.

use super::HtmlColorAndStyle;
use crate::color::{HColor, NoSuchColor};
use crate::java;
use crate::skin::SkinParam;
use crate::style::{PName, Style, ValueReading};

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Rainbow {
    colors: Vec<HtmlColorAndStyle>,
    /// The gap between the lines of a multi-colour arrow; with none, they are drawn thicker instead.
    color_arrow_separation_space: i32,
}

impl Rainbow {
    /// No colour: whoever draws the arrow picks its default.
    pub(crate) const fn none() -> Self {
        Self {
            colors: Vec::new(),
            color_arrow_separation_space: 0,
        }
    }

    /// One colour, its arrowhead in `arrow_head_color` or else the same; no colour without `arrow_color`
    /// (`fromColor`).
    pub(crate) fn from_color(
        arrow_color: Option<HColor>,
        arrow_head_color: Option<HColor>,
    ) -> Self {
        match arrow_color {
            None => Self::none(),
            Some(arrow_color) => Self::build(HtmlColorAndStyle::new(arrow_color, arrow_head_color)),
        }
    }

    /// The line colour a style gives arrows, and its head colour, which defaults to the line colour
    /// (`build(Style, HColorSet)`).
    pub(crate) fn build_from_style(style: &Style) -> Self {
        let color = style.value(PName::LineColor).as_color();
        let head = style.value(PName::HeadColor);
        let color_head = if head.is_none() {
            color.clone()
        } else {
            head.as_color()
        };
        Self::from_color(Some(color), Some(color_head))
    }

    /// One colour (`build(HtmlColorAndStyle)`).
    pub(crate) fn build(color: HtmlColorAndStyle) -> Self {
        Self {
            colors: vec![color],
            color_arrow_separation_space: 0,
        }
    }

    /// The colours of an arrow's specification, such as `#red;#blue,dashed`: one per `;`, each read by
    /// [`HtmlColorAndStyle::build`] (`build(ISkinParam, String, int)`).
    pub(crate) fn build_from_definition(
        skin_param: &SkinParam,
        color_string: &str,
        color_arrow_separation_space: i32,
    ) -> Result<Self, NoSuchColor> {
        let colors = java::split(color_string, ";")
            .iter()
            .map(|definition| HtmlColorAndStyle::build(skin_param, definition))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            colors,
            color_arrow_separation_space,
        })
    }

    /// These colours, or `default_color` when there are none (`withDefault`).
    #[must_use]
    pub(crate) fn with_default(&self, default_color: &Self) -> Self {
        if self.size() == 0 {
            default_color.clone()
        } else {
            self.clone()
        }
    }

    pub(crate) fn is_invisible(&self) -> bool {
        self.colors
            .iter()
            .any(|color| color.get_style().is_invisible())
    }

    pub(crate) fn get_colors(&self) -> &[HtmlColorAndStyle] {
        &self.colors
    }

    /// The first colour; a rainbow without colours has none, and PlantUML fails on asking.
    pub(crate) fn get_color(&self) -> &HColor {
        self.colors[0].get_arrow_color()
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
    use crate::decoration::LinkStyle;

    fn color(name: &str) -> HColor {
        HColor::parse(name).unwrap().unwrap()
    }

    #[test]
    fn a_specification_gives_one_colour_per_semicolon_with_its_line_style() {
        let skin = SkinParam::default();
        let rainbow = Rainbow::build_from_definition(&skin, "#red;#blue,dashed;bold", 2).unwrap();
        assert_eq!(rainbow.size(), 3);
        assert_eq!(rainbow.get_color_arrow_separation_space(), 2);
        let colors = rainbow.get_colors();
        assert_eq!(colors[0].get_arrow_color(), &color("red"));
        assert_eq!(colors[0].get_arrow_head_color(), &color("red"));
        assert!(colors[0].get_style().is_normal());
        assert_eq!(colors[1].get_arrow_color(), &color("blue"));
        assert_eq!(colors[1].get_style(), LinkStyle::DASHED);
        // A style alone keeps the arrow colour of the activity diagram's style.
        assert_eq!(colors[2].get_arrow_color(), &color("#181818"));
        assert_eq!(colors[2].get_style(), LinkStyle::BOLD);
        assert!(!rainbow.is_invisible());
    }

    #[test]
    fn hidden_colours_make_the_arrow_invisible_and_unknown_ones_fail() {
        let skin = SkinParam::default();
        assert!(
            Rainbow::build_from_definition(&skin, "#red;hidden", 0)
                .unwrap()
                .is_invisible()
        );
        assert!(Rainbow::build_from_definition(&skin, "#nosuchcolor", 0).is_err());
    }

    #[test]
    fn missing_colours_fall_back_to_the_default() {
        let red = Rainbow::from_color(Some(color("red")), None);
        assert_eq!(
            Rainbow::from_color(None, Some(color("blue"))),
            Rainbow::none()
        );
        assert_eq!(Rainbow::none().with_default(&red), red);
        let blue = Rainbow::from_color(Some(color("blue")), Some(color("green")));
        assert_eq!(blue.with_default(&red), blue);
        assert_eq!(blue.get_colors()[0].get_arrow_head_color(), &color("green"));
    }
}
