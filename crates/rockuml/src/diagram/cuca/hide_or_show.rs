//! One `hide`, `show`, `remove` or `restore` of entities by name, `<<stereotype>>`, `$tag` or `@unlinked`
//! (PlantUML's `HideOrShow`).

use super::CucaDiagram;
use crate::abel::{Entity, EntityId};
use crate::pattern::try_java_regex;
use crate::plasma::MAGIC_SEPARATOR;
use crate::stereo::Stereotype;

#[derive(Clone)]
pub(super) struct HideOrShow {
    what: String,
    show: bool,
}

impl HideOrShow {
    pub(super) fn new(what: String, show: bool) -> Self {
        Self { what, show }
    }

    fn is_applyable(&self, leaf: &Entity, diagram: &CucaDiagram) -> bool {
        assert!(!leaf.is_root(), "the root is never hidden");
        if let Some(tag) = self.what.strip_prefix('$') {
            return leaf
                .stereotags()
                .iter()
                .any(|stereotag| matches(&stereotag.name, tag));
        }
        if let Some(pattern) = self.stereotype_pattern() {
            return is_applyable_stereotype(leaf.stereotype.as_ref(), pattern);
        }
        if self.is_about_unlinked() {
            return leaf.is_alone_and_unlinked(diagram);
        }
        matches(
            diagram.quark(leaf.get_quark()).get_qualified_name(),
            &self.what,
        )
    }

    /// The pattern of `<<pattern>>`.
    fn stereotype_pattern(&self) -> Option<&str> {
        let pattern = self.what.strip_prefix("<<")?.strip_suffix(">>")?;
        Some(crate::java::trim(pattern))
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
        match self.stereotype_pattern() {
            Some(pattern) if is_applyable_stereotype(Some(stereotype), pattern) => !self.show,
            _ => hidden,
        }
    }
}

fn is_applyable_stereotype(stereotype: Option<&Stereotype>, pattern: &str) -> bool {
    stereotype.is_some_and(|stereotype| {
        stereotype
            .multiple_labels()
            .iter()
            .any(|label| matches(label, pattern))
    })
}

/// Whether the last part of `name` is `pattern`, where `*` matches anything.
fn matches(name: &str, pattern: &str) -> bool {
    let name = match name.rfind(MAGIC_SEPARATOR) {
        Some(idx) => &name[idx + MAGIC_SEPARATOR.len()..],
        None => name,
    };
    if pattern.contains('*') {
        let regex = format!("^{}$", pattern.replace('*', ".*"));
        return try_java_regex(&regex, false).is_some_and(|regex| regex.is_match(name));
    }
    name == pattern
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_match_exactly_or_with_stars() {
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
