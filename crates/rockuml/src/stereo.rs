//! Stereotypes like `<< Generated >>` or `<< (C,#ADD1B2) Testable >>`: labels shown in guillemets, and
//! an optional spot, a letter in a coloured circle (PlantUML's `Stereotype` and `StereotypeDecoration`).

use std::fmt::Write;
use std::sync::LazyLock;

use regex::Regex;

use crate::color::{HColor, NoSuchColor};
use crate::pattern::{RegexTree, java_regex};

/// The pattern stereotypes are written with, as an optional part of a command.
pub(crate) fn optional_pattern(name: &'static str) -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::spaces_zero_or_more(),
        RegexTree::optional(RegexTree::named(1, name, r"(\<\<.+?\>\>)")),
        RegexTree::spaces_zero_or_more(),
    ])
}

/// Tags like `$tag1 $tag2`, captured under `name` (`Stereotag.pattern`).
pub(crate) fn tags_pattern(name: &'static str) -> RegexTree {
    RegexTree::named(4, name, r"((\$[^%s{}%g<>$]+)([%s]+(\$[^%s{}%g<>$]+))*)?")
}

/// A letter in a coloured circle drawn before the labels.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Spot {
    pub character: char,
    pub color: Option<HColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Stereotype {
    /// The labels, each in `<<` and `>>`, without the spot.
    label: String,
    spot: Option<Spot>,
}

impl Stereotype {
    /// A stereotype as written, labels only (`Stereotype.build(label)`).
    pub(crate) fn new(label: &str) -> Self {
        Self {
            label: label.to_owned(),
            spot: None,
        }
    }

    /// A stereotype whose `(C,color)` parts become a spot (`Stereotype.build` with a circled font).
    pub(crate) fn with_spot(full: &str) -> Result<Self, NoSuchColor> {
        static CIRCLE_CHAR: LazyLock<Regex> = LazyLock::new(|| {
            java_regex(
                r"^\<\<[ \t]*\((\S)(?:[ \t]*,[ \t]*(#[0-9a-fA-F]{6}|\w+)[ \t]*)?\)(?:[,]?(.*?))?\>\>$",
                false,
            )
        });
        let mut label = String::new();
        let mut spot = None;
        for name in cut_labels(full) {
            match CIRCLE_CHAR.captures(&name) {
                Some(captures) => {
                    let color = captures
                        .get(2)
                        .map(|color| {
                            HColor::parse(color.as_str())
                                .ok()
                                .flatten()
                                .ok_or_else(|| NoSuchColor(color.as_str().to_owned()))
                        })
                        .transpose()?;
                    let character = captures[1].chars().next().expect("one character");
                    spot = Some(Spot { character, color });
                    if let Some(rest) = captures.get(3).filter(|rest| !is_blank(rest.as_str())) {
                        let _ = write!(label, "<<{}>>", rest.as_str());
                    }
                }
                None => label.push_str(&name),
            }
        }
        Ok(Self { label, spot })
    }

    pub(crate) fn spot(&self) -> Option<&Spot> {
        self.spot.as_ref()
    }

    /// The labels as shown, in guillemets (`getLabels(Guillemet.GUILLEMET)`).
    pub(crate) fn labels(&self) -> Vec<String> {
        cut_labels(&self.label)
            .iter()
            .map(|label| guillemets(label))
            .collect()
    }

    /// The labels as written, like `<<a>><<b>>` (`getLabel(Guillemet.DOUBLE_COMPARATOR)`).
    pub(crate) fn label_double_comparator(&self) -> &str {
        &self.label
    }

    /// Each label as written, like `<<a>>` (`getLabels(Guillemet.DOUBLE_COMPARATOR)`).
    pub(crate) fn labels_double_comparator(&self) -> Vec<String> {
        cut_labels(&self.label)
    }

    /// Each label's text, without brackets or the space next to them.
    pub(crate) fn multiple_labels(&self) -> Vec<String> {
        static LABEL: LazyLock<Regex> =
            LazyLock::new(|| java_regex(r"\<\<\s?((?:\<&\w+\>|[^<>])+?)\s?\>\>", false));
        LABEL
            .captures_iter(&self.label)
            .map(|captures| captures[1].to_owned())
            .collect()
    }

    /// The names style rules can select the stereotype by.
    pub(crate) fn style_names(&self) -> Vec<String> {
        cut_labels(&self.label)
            .iter()
            .map(|label| without_brackets(label).to_owned())
            .collect()
    }
}

/// As PlantUML writes it out, in tooltips for instance: the spot's letter, then the labels as written.
impl std::fmt::Display for Stereotype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.spot {
            Some(spot) => write!(f, "{} {}", spot.character, self.label),
            None => f.write_str(&self.label),
        }
    }
}

/// A `$tag` on an entity, named without its `$`, which `hide $tag` selects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Stereotag {
    pub name: String,
}

/// `StringUtils.isEmpty`: only spaces and tabs.
fn is_blank(text: &str) -> bool {
    text.chars().all(|c| matches!(c, ' ' | '\t'))
}

/// Every `<<label>>` in the text, skipping those written with three brackets.
fn cut_labels(label: &str) -> Vec<String> {
    static LABEL: LazyLock<Regex> = LazyLock::new(|| java_regex(r"\<{2,3}.*?\>{2,3}", false));
    LABEL
        .find_iter(label)
        .map(|found| found.as_str())
        .filter(|found| !found.starts_with("<<<"))
        .map(str::to_owned)
        .collect()
}

/// `Guillemet.manageGuillemetStrict` with `«` and `»`.
fn guillemets(label: &str) -> String {
    format!("\u{AB}{}\u{BB}", without_brackets(label))
}

fn without_brackets(label: &str) -> &str {
    let label = label
        .strip_prefix("<< ")
        .or_else(|| label.strip_prefix("<<"))
        .unwrap_or(label);
    label
        .strip_suffix(" >>")
        .or_else(|| label.strip_suffix(">>"))
        .unwrap_or(label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_shown_in_guillemets() {
        let stereotype = Stereotype::new("<< Generated >><<Big>>");
        assert_eq!(
            stereotype.labels(),
            ["\u{AB}Generated\u{BB}", "\u{AB}Big\u{BB}"]
        );
        assert_eq!(stereotype.style_names(), ["Generated", "Big"]);
    }

    #[test]
    fn labels_are_compared_as_written_or_bare() {
        let stereotype = Stereotype::new("<< Generated >><<Big>>");
        assert_eq!(
            stereotype.label_double_comparator(),
            "<< Generated >><<Big>>"
        );
        assert_eq!(
            stereotype.labels_double_comparator(),
            ["<< Generated >>", "<<Big>>"]
        );
        assert_eq!(stereotype.multiple_labels(), ["Generated", "Big"]);
    }

    #[test]
    fn spots_are_split_from_the_labels() {
        let stereotype = Stereotype::with_spot("<< (C,#ADD1B2) Testable >>").unwrap();
        assert_eq!(
            stereotype.spot(),
            Some(&Spot {
                character: 'C',
                color: HColor::parse("#ADD1B2").unwrap()
            })
        );
        assert_eq!(stereotype.labels(), ["\u{AB}Testable\u{BB}"]);
        let only_spot = Stereotype::with_spot("<< (D,orchid) >>").unwrap();
        assert_eq!(only_spot.labels(), Vec::<String>::new());
        assert_eq!(only_spot.to_string(), "D ");
    }

    #[test]
    fn written_out_with_the_spot_letter_first() {
        let stereotype = Stereotype::with_spot("<< (C,#ADD1B2) Testable >>").unwrap();
        assert_eq!(stereotype.to_string(), "C << Testable >>");
        let plain = Stereotype::new("<< Generated >>");
        assert_eq!(plain.to_string(), "<< Generated >>");
    }
}
