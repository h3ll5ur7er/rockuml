//! The words around a link: quantifiers and roles at its ends, and its label with the small arrow `<` or
//! `>` that may go with it (PlantUML's `Labels` and `StringWithArrow`).

use std::sync::LazyLock;

use regex::Regex;

use crate::abel::LinkArrow;
use crate::creole::Display;
use crate::diagram::cuca::CucaDiagram;
use crate::java;
use crate::pattern::{RegexResult, plantuml_regex};
use crate::text::unquoted;

pub(in crate::diagram) struct Labels {
    pub first_label: Option<String>,
    pub second_label: Option<String>,
    pub first_role: Option<String>,
    pub second_role: Option<String>,
    string_with_arrow: StringWithArrow,
}

impl Labels {
    pub(in crate::diagram) fn new(arg: &RegexResult) -> Self {
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
        let label_link = arg
            .get("LABEL_LINK", 0)
            .map(|label_link| labels.init(label_link));
        labels.string_with_arrow = StringWithArrow::new(label_link.as_deref());
        labels
    }

    /// `"1" label "*"`: quoted words at either end of the label are quantifiers, unless the arrow has some.
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
                self.second_label = None;
                return middle(&m[2]);
            }
            if let Some(m) = SECOND_LABEL_ONLY.captures(label_link) {
                self.first_label = None;
                self.second_label = Some(m[2].to_owned());
                return middle(&m[1]);
            }
        }
        unquoted(label_link).to_owned()
    }

    pub(in crate::diagram) fn get_link_arrow(&self) -> LinkArrow {
        self.string_with_arrow.link_arrow
    }

    /// The label without its arrow; `None` for a link without one.
    pub(in crate::diagram) fn get_display(&self) -> Option<Display> {
        self.string_with_arrow
            .label
            .as_deref()
            .map(Display::with_newlines)
    }
}

/// A label, and the arrow written before or after it.
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
        let complete_label = unquoted(complete_label);
        let (link_arrow, label) = if has_several_guide_lines(complete_label) {
            (LinkArrow::NoneOrSeveral, Some(complete_label))
        } else if complete_label == "<" {
            (LinkArrow::Backward, None)
        } else if complete_label == ">" {
            (LinkArrow::DirectNormal, None)
        } else if let Some(rest) = complete_label.strip_prefix("< ") {
            (LinkArrow::Backward, Some(java::trim(rest)))
        } else if let Some(rest) = complete_label.strip_prefix("> ") {
            (LinkArrow::DirectNormal, Some(java::trim(rest)))
        } else if let Some(rest) = complete_label.strip_suffix(" >") {
            (LinkArrow::DirectNormal, Some(java::trim(rest)))
        } else if let Some(rest) = complete_label.strip_suffix(" <") {
            (LinkArrow::Backward, Some(java::trim(rest)))
        } else {
            (LinkArrow::NoneOrSeveral, Some(complete_label))
        };
        Self {
            label: label.map(str::to_owned),
            link_arrow,
        }
    }
}

/// Whether the label has several lines and one of them has its own arrow.
fn has_several_guide_lines(s: &str) -> bool {
    let lines = java::split(s, r"\n");
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

    #[test]
    fn a_label_arrow_points_from_its_side() {
        let arrow = |label| StringWithArrow::new(Some(label));
        assert_eq!(arrow("places >").link_arrow, LinkArrow::DirectNormal);
        assert_eq!(arrow("places >").label.as_deref(), Some("places"));
        assert_eq!(arrow("< runs on").link_arrow, LinkArrow::Backward);
        assert_eq!(arrow("< runs on").label.as_deref(), Some("runs on"));
        assert_eq!(arrow(">").label, None);
        assert_eq!(arrow("plain").link_arrow, LinkArrow::NoneOrSeveral);
        assert_eq!(
            arrow(r"a >\nb").link_arrow,
            LinkArrow::NoneOrSeveral,
            "several lines keep their arrows"
        );
    }
}
