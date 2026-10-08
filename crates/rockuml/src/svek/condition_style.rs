//! How the conditions of activity diagrams are drawn: `skinparam conditionStyle` (PlantUML's
//! `ConditionStyle`) and `skinparam conditionEndStyle` (`ConditionEndStyle`).

/// The outline of a condition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConditionStyle {
    /// An empty diamond, the condition above it.
    EmptyDiamond,
    InsideHexagon,
    InsideDiamond,
}

impl ConditionStyle {
    /// The style a skinparam value names, ignoring case (`fromString`).
    pub(crate) fn from_string(value: &str) -> Option<Self> {
        let lower = value.to_ascii_lowercase();
        Some(match lower.as_str() {
            "insidediamond" | "foo1" | "inside_diamond" => Self::InsideDiamond,
            "diamond" | "empty_diamond" => Self::EmptyDiamond,
            "inside" | "inside_hexagon" => Self::InsideHexagon,
            _ => return None,
        })
    }
}

/// What ends a conditional.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConditionEndStyle {
    Diamond,
    /// A horizontal line joining the branches.
    Hline,
}

impl ConditionEndStyle {
    /// The style a skinparam value names, ignoring case (`fromString`).
    pub(crate) fn from_string(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "diamond" => Some(Self::Diamond),
            "hline" => Some(Self::Hline),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_are_named_as_in_plantuml_ignoring_case() {
        assert_eq!(
            ConditionStyle::from_string("InsideDiamond"),
            Some(ConditionStyle::InsideDiamond)
        );
        assert_eq!(
            ConditionStyle::from_string("FOO1"),
            Some(ConditionStyle::InsideDiamond)
        );
        assert_eq!(
            ConditionStyle::from_string("diamond"),
            Some(ConditionStyle::EmptyDiamond)
        );
        assert_eq!(
            ConditionStyle::from_string("empty_diamond"),
            Some(ConditionStyle::EmptyDiamond)
        );
        assert_eq!(
            ConditionStyle::from_string("Inside"),
            Some(ConditionStyle::InsideHexagon)
        );
        assert_eq!(ConditionStyle::from_string("hexagon"), None);
        assert_eq!(
            ConditionEndStyle::from_string("HLine"),
            Some(ConditionEndStyle::Hline)
        );
        assert_eq!(ConditionEndStyle::from_string("line"), None);
    }
}
