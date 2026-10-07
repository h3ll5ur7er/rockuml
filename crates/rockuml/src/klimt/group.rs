//! Named groups of drawn shapes, which SVG keeps as `<g>` elements (`UGroup`).

use crate::text::LineLocation;

/// In Java's declaration order, which is the order the attributes are written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UGroupType {
    Id,
    Class,
    /// A tooltip, which SVG writes as a `<title>` child rather than an attribute.
    Title,
    DataEntity,
    DataQualifiedName,
    DataEntity1,
    DataEntity2,
    DataEntityUid,
    DataEntity1Uid,
    DataEntity2Uid,
    DataParticipant,
    DataParticipant1,
    DataParticipant2,
    DataUid,
    DataSourceLine,
    DataVisibilityModifier,
    DataLinkType,
}

impl UGroupType {
    /// The attribute SVG writes the value in; `None` for the title and for what SVG leaves out
    /// (`PortableSvgDocument.applyGroupAttribute`).
    pub fn svg_attribute_name(self) -> Option<&'static str> {
        match self {
            Self::Class => Some("class"),
            Self::DataQualifiedName => Some("data-qualified-name"),
            // PlantUML writes the uid as the id, for now.
            Self::DataUid => Some("id"),
            Self::DataParticipant1 | Self::DataEntity1Uid => Some("data-entity-1"),
            Self::DataParticipant2 | Self::DataEntity2Uid => Some("data-entity-2"),
            Self::DataSourceLine => Some("data-source-line"),
            Self::DataEntityUid => Some("data-entity-uid"),
            Self::DataVisibilityModifier => Some("data-visibility-modifier"),
            Self::DataLinkType => Some("data-link-type"),
            Self::Id
            | Self::Title
            | Self::DataEntity
            | Self::DataEntity1
            | Self::DataEntity2
            | Self::DataParticipant => None,
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
