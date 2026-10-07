//! The colours one diagram element sets for itself, like `#pink;line:red;line.dashed;text:green`
//! (PlantUML's `Colors` and `ColorParser`).

use super::HColor;

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

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Colors {
    colors: Vec<(ColorType, HColor)>,
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
        Ok(result)
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
    }

    #[test]
    fn unknown_main_colours_are_errors() {
        assert_eq!(
            Colors::parse("#nosuchcolor", ColorType::Back),
            Err(NoSuchColor("nosuchcolor".to_owned()))
        );
    }
}
