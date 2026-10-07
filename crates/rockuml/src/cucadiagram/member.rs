//! One line of a class or object body, read as a field or a method: its modifiers, visibility and link
//! (PlantUML's `Member`).

use std::sync::LazyLock;

use regex::Regex;

use crate::creole::manage_guillemet;
use crate::java;
use crate::klimt::url::Url;
use crate::pattern::{java_regex, plantuml_regex};
use crate::skin::visibility_modifier::VisibilityModifier;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Member {
    display: String,
    static_modifier: bool,
    abstract_modifier: bool,
    url: Option<Url>,
    visibility_modifier: Option<VisibilityModifier>,
}

impl Member {
    pub(crate) fn method(raw: &str) -> Self {
        Self::new(raw, true)
    }

    pub(crate) fn field(raw: &str) -> Self {
        Self::new(raw, false)
    }

    fn new(raw: &str, is_method: bool) -> Self {
        static URL: LazyLock<Regex> =
            LazyLock::new(|| plantuml_regex(&format!(r"^(.*?)(?:\[({})\])?$", Url::regexp())));
        static REMOVE_TAG: LazyLock<Regex> =
            LazyLock::new(|| java_regex(r"(?i)\{(method|field)\}\s*", false));
        static REMOVE_STATIC_CLASSIFIER_ABSTRACT: LazyLock<Regex> =
            LazyLock::new(|| java_regex(r"(?i)\{(static|classifier|abstract)\}\s*", false));
        let without_tag = REMOVE_TAG.replace_all(raw, "");
        let captures = URL
            .captures(&without_tag)
            .expect("every line matches the optional link pattern");
        let text = captures.get(1).map_or("", |text| text.as_str());
        let url = captures.get(2).and_then(|url| Url::parse(url.as_str()));
        let lower = text.to_lowercase();
        let mut display_clean =
            java::trim(&REMOVE_STATIC_CLASSIFIER_ABSTRACT.replace_all(text, "")).to_owned();
        if display_clean.is_empty() {
            display_clean = " ".to_owned();
        }
        let visibility_modifier = VisibilityModifier::is_visibility_character(&display_clean)
            .then(|| VisibilityModifier::get_visibility_modifier(&display_clean, !is_method))
            .flatten();
        let display = if visibility_modifier.is_some() {
            let rest: String = display_clean.chars().skip(1).collect();
            java::trim(&manage_guillemet(&rest)).to_owned()
        } else {
            manage_guillemet(&display_clean)
        };
        Self {
            display,
            static_modifier: lower.contains("{static}") || lower.contains("{classifier}"),
            abstract_modifier: lower.contains("{abstract}"),
            url,
            visibility_modifier,
        }
    }

    /// The text shown, with the visibility as a character in front when there are no icons.
    pub(crate) fn get_display(&self, with_visibility_char: bool) -> String {
        if !with_visibility_char {
            return self.display.clone();
        }
        let prefix = match self.visibility_modifier {
            Some(VisibilityModifier::PrivateField | VisibilityModifier::PrivateMethod) => "-",
            Some(VisibilityModifier::PublicField | VisibilityModifier::PublicMethod) => "+",
            Some(
                VisibilityModifier::PackagePrivateField | VisibilityModifier::PackagePrivateMethod,
            ) => "~",
            Some(VisibilityModifier::ProtectedField | VisibilityModifier::ProtectedMethod) => "#",
            Some(VisibilityModifier::IeMandatory) => "*",
            None => "",
        };
        format!("{prefix}{}", self.display)
    }

    pub(crate) fn is_static(&self) -> bool {
        self.static_modifier
    }

    pub(crate) fn is_abstract(&self) -> bool {
        self.abstract_modifier
    }

    pub(crate) fn get_visibility_modifier(&self) -> Option<VisibilityModifier> {
        self.visibility_modifier
    }

    pub(crate) fn get_url(&self) -> Option<&Url> {
        self.url.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_and_visibility_leave_the_display() {
        let member = Member::method("{static} + count() : int");
        assert_eq!(member.get_display(false), "count() : int");
        assert_eq!(member.get_display(true), "+count() : int");
        assert!(member.is_static());
        assert!(!member.is_abstract());
        assert_eq!(
            member.get_visibility_modifier(),
            Some(VisibilityModifier::PublicMethod)
        );
        let field = Member::field("{abstract} # name <<id>>");
        assert_eq!(field.get_display(false), "name \u{AB}id\u{BB}");
        assert!(field.is_abstract());
        assert_eq!(
            field.get_visibility_modifier(),
            Some(VisibilityModifier::ProtectedField)
        );
        assert_eq!(Member::field("{field}  ").get_display(false), " ");
    }

    #[test]
    fn a_trailing_link_is_taken_off() {
        let member = Member::field("name [[[http://x.org]]]");
        assert_eq!(member.get_display(false), "name");
        assert_eq!(member.get_url().unwrap().href, "http://x.org");
    }
}
