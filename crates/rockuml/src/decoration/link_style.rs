//! How the line of a link is drawn: plain, dashed, dotted, bold or not at all, and how thick (PlantUML's
//! `LinkStyle`).

use crate::klimt::ugraphic::UStroke;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Type {
    Normal,
    Dashed,
    Dotted,
    Bold,
    Invisible,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LinkStyle {
    kind: Type,
    thickness: Option<f64>,
}

impl LinkStyle {
    const fn of(kind: Type) -> Self {
        Self {
            kind,
            thickness: None,
        }
    }

    pub(crate) const NORMAL: Self = Self::of(Type::Normal);
    pub(crate) const INVISIBLE: Self = Self::of(Type::Invisible);
    pub(crate) const BOLD: Self = Self::of(Type::Bold);
    pub(crate) const DOTTED: Self = Self::of(Type::Dotted);
    pub(crate) const DASHED: Self = Self::of(Type::Dashed);

    pub(crate) fn is_normal(self) -> bool {
        self.kind == Type::Normal
    }

    pub(crate) fn is_invisible(self) -> bool {
        self.kind == Type::Invisible
    }

    #[must_use]
    pub(crate) fn go_thickness(self, thickness: f64) -> Self {
        Self {
            thickness: Some(thickness),
            ..self
        }
    }

    pub(crate) fn get_stroke3(self) -> UStroke {
        let dashed = |dash_visible, dash_space| UStroke {
            dash_visible,
            dash_space,
            thickness: self.non_zero_thickness(),
        };
        match self.kind {
            Type::Dashed => dashed(7.0, 7.0),
            Type::Dotted => dashed(1.0, 3.0),
            Type::Bold => UStroke::with_thickness(2.0),
            Type::Normal | Type::Invisible => UStroke::with_thickness(self.non_zero_thickness()),
        }
    }

    fn non_zero_thickness(self) -> f64 {
        self.thickness.unwrap_or(1.0)
    }

    /// Like [`Self::from_string2`], but plain for any other word.
    pub(crate) fn from_string1(s: &str) -> Self {
        Self::from_string2(s).unwrap_or(Self::NORMAL)
    }

    /// `dashed`, `dotted`, `bold` or `hidden`, in any case.
    pub(crate) fn from_string2(s: &str) -> Option<Self> {
        [
            ("dashed", Self::DASHED),
            ("dotted", Self::DOTTED),
            ("bold", Self::BOLD),
            ("hidden", Self::INVISIBLE),
        ]
        .into_iter()
        .find(|(name, _)| s.eq_ignore_ascii_case(name))
        .map(|(_, style)| style)
    }

    pub(crate) fn is_thickness_overrided(self) -> bool {
        self.thickness.is_some()
    }
}

/// As PlantUML writes it out, like `DASHED(null)` or `BOLD(2.0)`.
impl std::fmt::Display for LinkStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self.kind {
            Type::Normal => "NORMAL",
            Type::Dashed => "DASHED",
            Type::Dotted => "DOTTED",
            Type::Bold => "BOLD",
            Type::Invisible => "INVISIBLE",
        };
        match self.thickness {
            Some(thickness) => write!(f, "{kind}({})", crate::java::double_to_string(thickness)),
            None => write!(f, "{kind}(null)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_are_read_from_their_names() {
        assert_eq!(LinkStyle::from_string2("DASHED"), Some(LinkStyle::DASHED));
        assert_eq!(
            LinkStyle::from_string2("hidden"),
            Some(LinkStyle::INVISIBLE)
        );
        assert_eq!(LinkStyle::from_string2("plain"), None);
        assert!(LinkStyle::from_string1("plain").is_normal());
    }

    #[test]
    fn strokes_follow_the_style_and_thickness() {
        assert_eq!(LinkStyle::DASHED.get_stroke3().to_string(), "7.0-7.0-1.0");
        assert_eq!(
            LinkStyle::DOTTED
                .go_thickness(2.5)
                .get_stroke3()
                .to_string(),
            "1.0-3.0-2.5"
        );
        assert_eq!(
            LinkStyle::BOLD.go_thickness(5.0).get_stroke3().to_string(),
            "0.0-0.0-2.0"
        );
        assert_eq!(LinkStyle::NORMAL.get_stroke3(), UStroke::SIMPLE);
    }
}
