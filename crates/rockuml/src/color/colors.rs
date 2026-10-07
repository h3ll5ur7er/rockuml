//! The colours one diagram element sets for itself, like `#pink;line:red;line.dashed;text:green`
//! (PlantUML's `Colors` and `ColorParser`).

use super::HColor;
use crate::decoration::LinkStyle;
use crate::klimt::ugraphic::UStroke;
use crate::pattern::RegexTree;
use crate::style::{PName, Style, ValueReading};

/// Which part of an element a colour paints.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ColorType {
    Text,
    Back,
    Header,
    Line,
    Arrow,
}

impl ColorType {
    fn named(name: &str) -> Option<Self> {
        match name {
            "text" => Some(Self::Text),
            "back" => Some(Self::Back),
            "header" => Some(Self::Header),
            "line" => Some(Self::Line),
            "arrow" => Some(Self::Arrow),
            _ => None,
        }
    }
}

/// An unknown colour name, which makes PlantUML reject the line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NoSuchColor(pub String);

/// The pattern of a colour specification, single colour or `;`-separated parts.
pub(crate) const COLORS_REGEXP: &str = concat!(
    r"(?:#(?:\w+[-\\|/]?\w+;)?(?:(?:text|back|header|line|line\.dashed|line\.dotted|line\.bold|shadowing)",
    r"(?::\w+[-\\|/]?\w+)?(?:;|(?![\w;:.])))+)|(?:#\w+[-\\|/]?\w+)"
);

/// An optional colour specification, captured under `name` (`ColorParser.simpleColor`).
pub(crate) fn optional_pattern(name: &'static str) -> RegexTree {
    RegexTree::named(1, name, format!("({COLORS_REGEXP})?"))
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Colors {
    colors: Vec<(ColorType, HColor)>,
    /// `line.dashed`, `line.dotted` or `line.bold`.
    line_style: Option<LinkStyle>,
}

impl Colors {
    /// Reads a specification; a part without a name paints `main_type`.
    pub(crate) fn parse(data: &str, main_type: ColorType) -> Result<Self, NoSuchColor> {
        let data = data.to_lowercase().replace('#', "");
        let mut result = Self::default();
        for part in data.split(';').filter(|part| !part.is_empty()) {
            match part.split_once(':') {
                None if !part.contains('.') => {
                    let color = HColor::parse(part)
                        .ok()
                        .flatten()
                        .ok_or_else(|| NoSuchColor(part.to_owned()))?;
                    result.put(main_type, color);
                }
                None | Some(("shadowing", _)) => {}
                Some((name, value)) => {
                    // PlantUML stores an unknown colour as missing rather than failing.
                    if let (Some(kind), Some(color)) =
                        (ColorType::named(name), HColor::parse(value).ok().flatten())
                    {
                        result.put(kind, color);
                    }
                }
            }
        }
        result.line_style = [
            ("line.dashed", LinkStyle::DASHED),
            ("line.dotted", LinkStyle::DOTTED),
            ("line.bold", LinkStyle::BOLD),
        ]
        .into_iter()
        .find(|(name, _)| data.contains(name))
        .map(|(_, style)| style);
        Ok(result)
    }

    /// The line style of an old `##[dashed]` specification, any other word drawing plain lines.
    #[must_use]
    pub(crate) fn add_legacy_stroke(&self, style: &str) -> Self {
        Self {
            line_style: Some(LinkStyle::from_string1(style)),
            ..self.clone()
        }
    }

    /// The stroke of the line style, if there is one.
    pub(crate) fn get_specific_line_stroke(&self) -> Option<UStroke> {
        self.line_style.map(LinkStyle::get_stroke3)
    }

    fn put(&mut self, kind: ColorType, color: HColor) {
        match self
            .colors
            .iter_mut()
            .find(|(existing, _)| *existing == kind)
        {
            Some(entry) => entry.1 = color,
            None => self.colors.push((kind, color)),
        }
    }

    #[must_use]
    pub(crate) fn with(&self, kind: ColorType, color: Option<HColor>) -> Self {
        let mut result = self.clone();
        if let Some(color) = color {
            result.put(kind, color);
        }
        result
    }

    /// These colours, overridden by those `other` sets.
    #[must_use]
    pub(crate) fn merge_with(&self, other: &Self) -> Self {
        let mut result = self.clone();
        for (kind, color) in &other.colors {
            result.put(*kind, color.clone());
        }
        if other.line_style.is_some() {
            result.line_style = other.line_style;
        }
        result
    }

    /// The background or line colour set here, or else the style's (`getColor(Style, PName, HColorSet)`).
    pub(crate) fn get_color_of(&self, style: &Style, name: PName) -> HColor {
        let own = match name {
            PName::BackGroundColor => self.get(ColorType::Back),
            PName::LineColor => self.get(ColorType::Line),
            _ => None,
        };
        own.cloned().unwrap_or_else(|| style.value(name).as_color())
    }

    pub(crate) fn get(&self, kind: ColorType) -> Option<&HColor> {
        self.colors
            .iter()
            .find(|(existing, _)| *existing == kind)
            .map(|(_, color)| color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color(name: &str) -> HColor {
        HColor::parse(name).unwrap().unwrap()
    }

    #[test]
    fn a_single_colour_paints_the_main_part() {
        let colors = Colors::parse("#Pink", ColorType::Back).unwrap();
        assert_eq!(colors.get(ColorType::Back), Some(&color("pink")));
        assert_eq!(colors.get(ColorType::Line), None);
    }

    #[test]
    fn named_parts_are_read_and_line_styles_skipped() {
        let colors =
            Colors::parse("#pink;line:red;line.dashed;text:green", ColorType::Back).unwrap();
        assert_eq!(colors.get(ColorType::Back), Some(&color("pink")));
        assert_eq!(colors.get(ColorType::Line), Some(&color("red")));
        assert_eq!(colors.get(ColorType::Text), Some(&color("green")));
        assert_eq!(
            colors.get_specific_line_stroke(),
            Some(LinkStyle::DASHED.get_stroke3())
        );
    }

    #[test]
    fn merging_keeps_the_other_colours_and_line_style() {
        let base = Colors::parse("#pink;line:red", ColorType::Back).unwrap();
        let other = Colors::parse("#blue", ColorType::Back)
            .unwrap()
            .add_legacy_stroke("dotted");
        let merged = base.merge_with(&other);
        assert_eq!(merged.get(ColorType::Back), Some(&color("blue")));
        assert_eq!(merged.get(ColorType::Line), Some(&color("red")));
        assert_eq!(
            merged.get_specific_line_stroke(),
            Some(LinkStyle::DOTTED.get_stroke3())
        );
    }

    #[test]
    fn unknown_main_colours_are_errors() {
        assert_eq!(
            Colors::parse("#nosuchcolor", ColorType::Back),
            Err(NoSuchColor("nosuchcolor".to_owned()))
        );
    }
}
