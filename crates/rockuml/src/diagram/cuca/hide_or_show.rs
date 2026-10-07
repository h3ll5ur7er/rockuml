//! One `hide`, `show`, `remove` or `restore` of entities by name, `<<stereotype>>`, `$tag` or `@unlinked`
//! (PlantUML's `HideOrShow`).

use regex::Regex;

use super::CucaDiagram;
use crate::abel::{Entity, EntityId};
use crate::pattern::try_java_regex;
use crate::plasma::MAGIC_SEPARATOR;
use crate::stereo::Stereotype;

#[derive(Clone)]
pub(super) struct HideOrShow {
    what: String,
    show: bool,
    /// What names must be to match: `*` matches anything.
    pattern: NamePattern,
}

/// A name, or a pattern where `*` matches anything, compiled once.
#[derive(Clone)]
enum NamePattern {
    Exact(String),
    /// `None` for a pattern Java's regexes refuse, which matches nothing.
    Wildcard(Option<Regex>),
}

impl NamePattern {
    fn new(pattern: &str) -> Self {
        if pattern.contains('*') {
            let regex = format!("^{}$", pattern.replace('*', ".*"));
            Self::Wildcard(try_java_regex(&regex, false))
        } else {
            Self::Exact(pattern.to_owned())
        }
    }

    /// Whether the last part of `name` matches.
    fn matches(&self, name: &str) -> bool {
        let name = match name.rfind(MAGIC_SEPARATOR) {
            Some(idx) => &name[idx + MAGIC_SEPARATOR.len()..],
            None => name,
        };
        match self {
            Self::Exact(pattern) => name == pattern,
            Self::Wildcard(regex) => regex.as_ref().is_some_and(|regex| regex.is_match(name)),
        }
    }
}

impl HideOrShow {
    pub(super) fn new(what: String, show: bool) -> Self {
        let pattern = what
            .strip_prefix('$')
            .or_else(|| stereotype_pattern(&what))
            .unwrap_or(&what);
        Self {
            pattern: NamePattern::new(pattern),
            what,
            show,
        }
    }

    fn is_applyable(&self, leaf: &Entity, diagram: &CucaDiagram) -> bool {
        assert!(!leaf.is_root(), "the root is never hidden");
        if self.what.starts_with('$') {
            return leaf
                .stereotags()
                .iter()
                .any(|stereotag| self.pattern.matches(&stereotag.name));
        }
        if stereotype_pattern(&self.what).is_some() {
            return self.is_applyable_stereotype(leaf.stereotype.as_ref());
        }
        if self.is_about_unlinked() {
            return leaf.is_alone_and_unlinked(diagram);
        }
        self.pattern
            .matches(diagram.quark(leaf.get_quark()).get_qualified_name())
    }

    fn is_applyable_stereotype(&self, stereotype: Option<&Stereotype>) -> bool {
        stereotype.is_some_and(|stereotype| {
            stereotype
                .multiple_labels()
                .iter()
                .any(|label| self.pattern.matches(label))
        })
    }

    pub(super) fn is_about_unlinked(&self) -> bool {
        self.what.eq_ignore_ascii_case("@unlinked")
    }

    /// Whether `leaf` is hidden (or removed) once this command is applied over the earlier ones' `hidden`.
    pub(super) fn apply(&self, hidden: bool, leaf: EntityId, diagram: &CucaDiagram) -> bool {
        if self.is_applyable(diagram.entity(leaf), diagram) {
            return !self.show;
        }
        hidden
    }

    pub(super) fn apply_to_stereotype(&self, hidden: bool, stereotype: &Stereotype) -> bool {
        if stereotype_pattern(&self.what).is_some()
            && self.is_applyable_stereotype(Some(stereotype))
        {
            return !self.show;
        }
        hidden
    }
}

/// The pattern of `<<pattern>>`.
fn stereotype_pattern(what: &str) -> Option<&str> {
    let pattern = what.strip_prefix("<<")?.strip_suffix(">>")?;
    Some(crate::java::trim(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_match_exactly_or_with_stars() {
        let matches = |name: &str, pattern: &str| NamePattern::new(pattern).matches(name);
        assert!(matches("Foo", "Foo"));
        assert!(!matches("Foo", "foo"));
        assert!(matches("FooBar", "Foo*"));
        assert!(matches("a\u{1}FooBar", "*Bar"));
        assert!(!matches("a.FooBar", "Foo*"));
    }

    #[test]
    fn stereotypes_match_their_bare_labels() {
        let hide = HideOrShow::new("<< Serial* >>".to_owned(), false);
        let stereotype = Stereotype::new("<<Big>><<Serializable>>");
        assert!(hide.apply_to_stereotype(false, &stereotype));
        assert!(!hide.apply_to_stereotype(false, &Stereotype::new("<<Big>>")));
        let show = HideOrShow::new("<<Big>>".to_owned(), true);
        assert!(!show.apply_to_stereotype(true, &stereotype));
    }
}
