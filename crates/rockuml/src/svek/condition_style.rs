//! How the tests of activity diagrams are drawn: `skinparam conditionStyle` (PlantUML's
//! `svek.ConditionStyle`).

use crate::skin::SkinParam;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConditionStyle {
    /// A small diamond with the test beside it.
    EmptyDiamond,
    /// The test inside a hexagon.
    InsideHexagon,
    /// The test inside a diamond.
    InsideDiamond,
}

impl ConditionStyle {
    pub(crate) fn from_string(value: &str) -> Option<Self> {
        let lowercase = value.to_ascii_lowercase();
        match lowercase.as_str() {
            "insidediamond" | "foo1" | "inside_diamond" => Some(Self::InsideDiamond),
            "diamond" | "empty_diamond" => Some(Self::EmptyDiamond),
            "inside" | "inside_hexagon" => Some(Self::InsideHexagon),
            _ => None,
        }
    }
}

impl SkinParam {
    /// `skinparam conditionStyle`, a hexagon when it names no style.
    pub(crate) fn get_condition_style(&self) -> ConditionStyle {
        self.value("conditionStyle")
            .and_then(|value| ConditionStyle::from_string(&value))
            .unwrap_or(ConditionStyle::InsideHexagon)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_are_named_in_any_case_or_by_their_constant() {
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
            ConditionStyle::from_string("Empty_Diamond"),
            Some(ConditionStyle::EmptyDiamond)
        );
        assert_eq!(
            ConditionStyle::from_string("inside"),
            Some(ConditionStyle::InsideHexagon)
        );
        assert_eq!(ConditionStyle::from_string("hexagon"), None);
        assert_eq!(
            SkinParam::default().get_condition_style(),
            ConditionStyle::InsideHexagon
        );
    }
}
