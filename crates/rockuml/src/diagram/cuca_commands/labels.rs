//! The texts written around a link: the labels at its ends and the label in its middle, with the small
//! arrow a `<` or `>` asks for (PlantUML's `Labels` and `StringWithArrow`).

use std::sync::LazyLock;

use regex::Regex;

use crate::abel::LinkArrow;
use crate::diagram::cuca::CucaDiagram;
use crate::java;
use crate::pattern::{RegexResult, plantuml_regex};
use crate::text::unquoted;

pub(crate) struct Labels {
    first_label: Option<String>,
    second_label: Option<String>,
    first_role: Option<String>,
    second_role: Option<String>,
    string_with_arrow: StringWithArrow,
}

impl Labels {
    /// The labels of `FIRST_LABEL`, `SECOND_LABEL` and `LABEL_LINK`; a middle label like `"1" calls "*"`
    /// gives the end labels when the ends have none.
    pub(crate) fn new(arg: &RegexResult) -> Self {
        let mut labels = Self {
            first_label: arg.get("FIRST_LABEL", 0).map(str::to_owned),
            second_label: arg.get("SECOND_LABEL", 0).map(str::to_owned),
            first_role: arg
                .get("FIRST_ROLE", 0)
                .map(|role| unquoted(role).to_owned()),
            second_role: arg
                .get("SECOND_ROLE", 0)
                .map(|role| unquoted(role).to_owned()),
            string_with_arrow: StringWithArrow::new(None),
        };
        let label_link = arg.get("LABEL_LINK", 0).map(|label| labels.init(label));
        labels.string_with_arrow = StringWithArrow::new(label_link.as_deref());
        labels
    }

    fn init(&mut self, label_link: &str) -> String {
        static BOTH_LABELS: LazyLock<Regex> =
            LazyLock::new(|| plantuml_regex("^[%g]([^%g]+)[%g]([^%g]+)[%g]([^%g]+)[%g]$"));
        static FIRST_LABEL_ONLY: LazyLock<Regex> =
            LazyLock::new(|| plantuml_regex("^[%g]([^%g]+)[%g]([^%g]+)$"));
        static SECOND_LABEL_ONLY: LazyLock<Regex> =
            LazyLock::new(|| plantuml_regex("^([^%g]+)[%g]([^%g]+)[%g]$"));
        let middle = |text: &str| java::trim(CucaDiagram::clean_id(java::trim(text))).to_owned();
        if self.first_label.is_none() && self.second_label.is_none() {
            if let Some(m) = BOTH_LABELS.captures(label_link) {
                self.first_label = Some(m[1].to_owned());
                self.second_label = Some(m[3].to_owned());
                return middle(&m[2]);
            }
            if let Some(m) = FIRST_LABEL_ONLY.captures(label_link) {
                self.first_label = Some(m[1].to_owned());
                return middle(&m[2]);
            }
            if let Some(m) = SECOND_LABEL_ONLY.captures(label_link) {
                self.second_label = Some(m[2].to_owned());
                return middle(&m[1]);
            }
        }
        unquoted_strictly(label_link).to_owned()
    }

    pub(crate) fn get_first_label(&self) -> Option<String> {
        self.first_label.clone()
    }

    pub(crate) fn get_second_label(&self) -> Option<String> {
        self.second_label.clone()
    }

    /// The role written after `/` at the link's first end.
    pub(crate) fn get_first_role(&self) -> Option<String> {
        self.first_role.clone()
    }

    pub(crate) fn get_second_role(&self) -> Option<String> {
        self.second_role.clone()
    }

    pub(crate) fn get_label_link(&self) -> Option<&str> {
        self.string_with_arrow.label.as_deref()
    }

    pub(crate) fn get_link_arrow(&self) -> LinkArrow {
        self.string_with_arrow.link_arrow
    }
}

/// A label that may start or end with `<` or `>`, which become a small arrow beside it.
struct StringWithArrow {
    label: Option<String>,
    link_arrow: LinkArrow,
}

impl StringWithArrow {
    fn new(complete_label: Option<&str>) -> Self {
        let Some(complete_label) = complete_label else {
            return Self {
                label: None,
                link_arrow: LinkArrow::NoneOrSeveral,
            };
        };
        let complete_label = unquoted_strictly(complete_label);
        let labelled = |link_arrow, label: Option<&str>| Self {
            label: label.map(str::to_owned),
            link_arrow,
        };
        if has_several_guide_lines(complete_label) {
            return labelled(LinkArrow::NoneOrSeveral, Some(complete_label));
        }
        match complete_label {
            "<" => labelled(LinkArrow::Backward, None),
            ">" => labelled(LinkArrow::DirectNormal, None),
            _ => {
                if let Some(rest) = complete_label.strip_prefix("< ") {
                    labelled(LinkArrow::Backward, Some(java::trim(rest)))
                } else if let Some(rest) = complete_label.strip_prefix("> ") {
                    labelled(LinkArrow::DirectNormal, Some(java::trim(rest)))
                } else if let Some(rest) = complete_label.strip_suffix(" >") {
                    labelled(LinkArrow::DirectNormal, Some(java::trim(rest)))
                } else if let Some(rest) = complete_label.strip_suffix(" <") {
                    labelled(LinkArrow::Backward, Some(java::trim(rest)))
                } else {
                    labelled(LinkArrow::NoneOrSeveral, Some(complete_label))
                }
            }
        }
    }
}

/// `eventuallyRemoveStartingAndEndingDoubleQuote(s, "\"")`.
fn unquoted_strictly(text: &str) -> &str {
    if text.chars().count() < 2 {
        text
    } else {
        unquoted(text)
    }
}

/// Whether the label has several lines, one of which has its own small arrow.
fn has_several_guide_lines(label: &str) -> bool {
    let lines = java::split(label, "\\n");
    lines.len() > 1
        && lines.iter().any(|line| {
            line.starts_with("< ")
                || line.starts_with("> ")
                || line.ends_with(" <")
                || line.ends_with(" >")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn string_with_arrow(label: &str) -> (Option<String>, LinkArrow) {
        let result = StringWithArrow::new(Some(label));
        (result.label, result.link_arrow)
    }

    #[test]
    fn arrows_beside_the_label_are_taken_off() {
        assert_eq!(
            string_with_arrow("uses >"),
            (Some("uses".to_owned()), LinkArrow::DirectNormal)
        );
        assert_eq!(
            string_with_arrow("< calls"),
            (Some("calls".to_owned()), LinkArrow::Backward)
        );
        assert_eq!(string_with_arrow("<"), (None, LinkArrow::Backward));
        assert_eq!(
            string_with_arrow("\"plain\""),
            (Some("plain".to_owned()), LinkArrow::NoneOrSeveral)
        );
        assert_eq!(
            string_with_arrow("a >\\nb"),
            (Some("a >\\nb".to_owned()), LinkArrow::NoneOrSeveral)
        );
    }
}
