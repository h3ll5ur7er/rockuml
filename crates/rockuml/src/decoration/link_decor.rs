//! The decoration drawn at one end of a link, like an arrow head or a diamond (PlantUML's `LinkDecor`,
//! without the extremity factories).

use crate::java;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LinkDecor {
    None,
    Extends,
    Composition,
    Aggregation,
    NotNavigable,
    Redefines,
    DefinedBy,
    Crowfoot,
    CircleCrowfoot,
    CircleLine,
    DoubleLine,
    LineCrowfoot,
    Arrow,
    ArrowTriangle,
    ArrowAndCircle,
    Circle,
    CircleFill,
    CircleConnect,
    Parenthesis,
    Square,
    CircleCross,
    Plus,
    HalfArrowUp,
    HalfArrowDown,
}

impl LinkDecor {
    /// In declaration order, which the decoration patterns keep among keys of the same length.
    const ALL: [Self; 24] = [
        Self::None,
        Self::Extends,
        Self::Composition,
        Self::Aggregation,
        Self::NotNavigable,
        Self::Redefines,
        Self::DefinedBy,
        Self::Crowfoot,
        Self::CircleCrowfoot,
        Self::CircleLine,
        Self::DoubleLine,
        Self::LineCrowfoot,
        Self::Arrow,
        Self::ArrowTriangle,
        Self::ArrowAndCircle,
        Self::Circle,
        Self::CircleFill,
        Self::CircleConnect,
        Self::Parenthesis,
        Self::Square,
        Self::CircleCross,
        Self::Plus,
        Self::HalfArrowUp,
        Self::HalfArrowDown,
    ];

    /// How the decoration is written at the start of an arrow, like `<|` in `<|--`.
    fn decors1(self) -> &'static [&'static str] {
        match self {
            Self::Extends => &["<|", "^"],
            Self::Composition => &["*"],
            Self::Aggregation => &["o"],
            Self::NotNavigable => &["x"],
            Self::Redefines => &["<||"],
            Self::DefinedBy => &["<|:"],
            Self::Crowfoot => &["}"],
            Self::CircleCrowfoot => &["}o"],
            Self::CircleLine => &["|o"],
            Self::DoubleLine => &["||"],
            Self::LineCrowfoot => &["}|"],
            Self::Arrow => &["<", "<_"],
            Self::ArrowTriangle => &["<<"],
            Self::Circle => &["0"],
            Self::CircleFill => &["@"],
            Self::CircleConnect => &["0)"],
            Self::Parenthesis => &[")"],
            Self::Square => &["#"],
            Self::Plus => &["+"],
            Self::None
            | Self::ArrowAndCircle
            | Self::CircleCross
            | Self::HalfArrowUp
            | Self::HalfArrowDown => &[],
        }
    }

    /// How the decoration is written at the end of an arrow, like `|>` in `--|>`.
    fn decors2(self) -> &'static [&'static str] {
        match self {
            Self::Extends => &["|>", "^"],
            Self::Composition => &["*"],
            Self::Aggregation => &["o"],
            Self::NotNavigable => &["x"],
            Self::Redefines => &["||>"],
            Self::DefinedBy => &[":|>"],
            Self::Crowfoot => &["{"],
            Self::CircleCrowfoot => &["o{"],
            Self::CircleLine => &["o|"],
            Self::DoubleLine => &["||"],
            Self::LineCrowfoot => &["|{"],
            Self::Arrow => &[">", "_>"],
            Self::ArrowTriangle => &[">>"],
            Self::Circle => &["0"],
            Self::CircleFill => &["@"],
            Self::CircleConnect => &["(0"],
            Self::Parenthesis => &["("],
            Self::Square => &["#"],
            Self::Plus => &["+"],
            Self::HalfArrowUp => &["\\\\"],
            Self::HalfArrowDown => &["//"],
            Self::None | Self::ArrowAndCircle | Self::CircleCross => &[],
        }
    }

    /// Whether the decoration is filled with the line colour.
    pub(crate) fn is_fill(self) -> bool {
        matches!(
            self,
            Self::Composition | Self::Crowfoot | Self::Arrow | Self::ArrowTriangle
        )
    }

    /// The decoration written `s` at the start of an arrow; `None` for anything else.
    pub(crate) fn lookup_decors1(s: Option<&str>) -> Self {
        Self::lookup(s, Self::decors1)
    }

    /// The decoration written `s` at the end of an arrow; `None` for anything else.
    pub(crate) fn lookup_decors2(s: Option<&str>) -> Self {
        Self::lookup(s, Self::decors2)
    }

    fn lookup(s: Option<&str>, decors: fn(Self) -> &'static [&'static str]) -> Self {
        let Some(s) = s else {
            return Self::None;
        };
        let s = java::trim(s);
        Self::ALL
            .into_iter()
            .find(|decor| decors(*decor).contains(&s))
            .unwrap_or(Self::None)
    }

    /// The optional pattern of every start decoration.
    pub(crate) fn get_regex_decors1() -> String {
        Self::build_regex_from_decor_keys(Self::decors1)
    }

    /// The optional pattern of every end decoration.
    pub(crate) fn get_regex_decors2() -> String {
        Self::build_regex_from_decor_keys(Self::decors2)
    }

    /// Longest keys first, so that `||` is tried before `|`. PlantUML takes its keys from a `HashMap`, but
    /// two keys of the same length never match the same text, so their order does not matter.
    fn build_regex_from_decor_keys(decors: fn(Self) -> &'static [&'static str]) -> String {
        let mut keys: Vec<&str> = Self::ALL.into_iter().flat_map(decors).copied().collect();
        keys.sort_by_key(|key| std::cmp::Reverse(key.len()));
        let alternatives: Vec<String> = keys
            .into_iter()
            .map(|key| {
                let quoted = format!("\\Q{key}\\E");
                match (key.starts_with('o'), key.ends_with('o')) {
                    (true, true) => format!("\\b{quoted}\\b"),
                    (true, false) => format!("\\b{quoted}"),
                    (false, true) => format!("{quoted}\\b"),
                    (false, false) => quoted,
                }
            })
            .collect();
        format!("({})?", alternatives.join("|"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::java_regex;

    /// Splits an arrow like `<|--` into its start decoration, body and end decoration, the way the link
    /// commands' patterns do.
    fn decors_of(arrow: &str) -> (LinkDecor, LinkDecor) {
        let pattern = format!(
            "^{}(?:-+|\\.+|=+){}$",
            LinkDecor::get_regex_decors1(),
            LinkDecor::get_regex_decors2()
        );
        let regex = java_regex(&pattern, false);
        let captures = regex
            .captures(arrow)
            .unwrap_or_else(|| panic!("{arrow} is an arrow"));
        (
            LinkDecor::lookup_decors1(captures.get(1).map(|m| m.as_str())),
            LinkDecor::lookup_decors2(captures.get(2).map(|m| m.as_str())),
        )
    }

    #[test]
    fn arrows_read_as_their_decorations() {
        use LinkDecor::{Aggregation, Arrow, Composition, Crowfoot, Extends, Plus, Square};
        let none = LinkDecor::None;
        assert_eq!(decors_of("-->"), (none, Arrow));
        assert_eq!(decors_of("<|--"), (Extends, none));
        assert_eq!(decors_of("*-->"), (Composition, Arrow));
        assert_eq!(decors_of("..>"), (none, Arrow));
        assert_eq!(decors_of("o--"), (Aggregation, none));
        assert_eq!(decors_of("#--"), (Square, none));
        assert_eq!(decors_of("}--"), (Crowfoot, none));
        assert_eq!(decors_of("+--"), (Plus, none));
        assert_eq!(decors_of("^--"), (Extends, none));
        assert_eq!(decors_of("--|>"), (none, Extends));
        assert_eq!(
            decors_of("<||--||>"),
            (LinkDecor::Redefines, LinkDecor::Redefines)
        );
        assert_eq!(
            decors_of("}o--o{"),
            (LinkDecor::CircleCrowfoot, LinkDecor::CircleCrowfoot)
        );
    }

    #[test]
    fn unknown_or_missing_decorations_are_none() {
        assert_eq!(LinkDecor::lookup_decors1(None), LinkDecor::None);
        assert_eq!(LinkDecor::lookup_decors1(Some("|>")), LinkDecor::None);
        assert_eq!(LinkDecor::lookup_decors2(Some(" |> ")), LinkDecor::Extends);
    }

    #[test]
    fn an_o_ending_a_name_is_no_aggregation() {
        let regex = java_regex(
            &format!("^(\\w+?){}--$", LinkDecor::get_regex_decors1()),
            false,
        );
        let head_and_name = |text| {
            let captures = regex.captures(text).expect("a name and an arrow");
            (
                captures[1].to_owned(),
                captures.get(2).map(|m| m.as_str().to_owned()),
            )
        };
        assert_eq!(head_and_name("fooo--"), ("fooo".to_owned(), None));
        assert_eq!(
            head_and_name("foo*--"),
            ("foo".to_owned(), Some("*".to_owned()))
        );
    }

    #[test]
    fn arrows_and_diamonds_are_filled() {
        assert!(LinkDecor::Composition.is_fill());
        assert!(!LinkDecor::Aggregation.is_fill());
    }
}
