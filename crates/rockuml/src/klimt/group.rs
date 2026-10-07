//! Named groups of drawn shapes, which SVG keeps as `<g>` elements (`UGroup`).

use crate::text::LineLocation;

/// In Java's declaration order, which is the order the attributes are written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UGroupType {
    Class,
    /// A tooltip, which SVG writes as a `<title>` child rather than an attribute.
    Title,
    DataSourceLine,
}

impl UGroupType {
    pub fn svg_attribute_name(self) -> Option<&'static str> {
        match self {
            Self::Class => Some("class"),
            Self::Title => None,
            Self::DataSourceLine => Some("data-source-line"),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UGroup {
    entries: Vec<(UGroupType, String)>,
}

impl UGroup {
    /// A group remembering the source line its element was written on, if known.
    pub fn at(location: Option<&LineLocation>) -> Self {
        let mut group = Self::default();
        if let Some(location) = location {
            group.put(UGroupType::DataSourceLine, &location.position().to_string());
        }
        group
    }

    pub fn singleton(kind: UGroupType, value: &str) -> Self {
        let mut group = Self::default();
        group.put(kind, value);
        group
    }

    /// Characters other than letters, digits, `_`, `-` and spaces become dots.
    pub fn put(&mut self, kind: UGroupType, value: &str) {
        let value: String = value
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | ' ') {
                    c
                } else {
                    '.'
                }
            })
            .collect();
        self.entries.retain(|(known, _)| *known != kind);
        self.entries.push((kind, value));
        self.entries.sort_by_key(|(kind, _)| *kind);
    }

    pub fn entries(&self) -> impl Iterator<Item = (UGroupType, &str)> {
        self.entries
            .iter()
            .map(|(kind, value)| (*kind, value.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_come_in_declaration_order_with_odd_characters_replaced() {
        let mut group = UGroup::at(Some(&LineLocation::new("x", None).one_line_read()));
        group.put(UGroupType::Class, "a<b> c-d");
        let entries: Vec<_> = group.entries().collect();
        assert_eq!(
            entries,
            [
                (UGroupType::Class, "a.b. c-d"),
                (UGroupType::DataSourceLine, "0")
            ]
        );
    }
}
