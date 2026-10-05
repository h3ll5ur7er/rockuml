use std::collections::BTreeSet;

use super::names::{SName, SNames};

/// What a style rule applies to, or what an element is: selector names, stereotypes, a tree depth, and
/// whether the rule reaches deeper (`*`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct StyleSignature {
    names: SNames,
    /// `depth(n)`: -1 when not given.
    level: i32,
    starred: bool,
    stereotypes: BTreeSet<String>,
}

impl StyleSignature {
    pub fn empty() -> Self {
        Self {
            level: -1,
            ..Self::default()
        }
    }

    pub fn of(names: &[SName]) -> Self {
        Self {
            names: SNames::of(names),
            ..Self::empty()
        }
    }

    #[must_use]
    pub fn with_name(&self, name: SName) -> Self {
        Self {
            names: self.names.with(name),
            ..self.clone()
        }
    }

    /// Stereotypes are compared lowercase and without `_` or `.`, so `.Foo_Bar` matches `<<foobar>>`.
    #[must_use]
    pub fn with_stereotype(&self, stereotype: &str) -> Self {
        let cleaned: String = stereotype
            .chars()
            .filter(|c| *c != '_' && *c != '.')
            .flat_map(char::to_lowercase)
            .collect();
        let mut stereotypes = self.stereotypes.clone();
        stereotypes.insert(cleaned);
        Self {
            stereotypes,
            ..self.clone()
        }
    }

    #[must_use]
    pub fn with_level(&self, level: i32) -> Self {
        Self {
            level,
            ..self.clone()
        }
    }

    #[must_use]
    pub fn with_star(&self) -> Self {
        Self {
            starred: true,
            ..self.clone()
        }
    }

    pub fn has_stereotypes(&self) -> bool {
        !self.stereotypes.is_empty()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty() && self.stereotypes.is_empty()
    }

    /// The signature of `self` and `other` merged into one rule.
    #[must_use]
    pub fn merge_with(&self, other: &StyleSignature) -> Self {
        Self {
            names: self.names.union(other.names),
            level: self.level.max(other.level),
            starred: self.starred || other.starred,
            stereotypes: self
                .stereotypes
                .union(&other.stereotypes)
                .cloned()
                .collect(),
        }
    }

    /// Whether a rule with this signature applies to `element`.
    pub fn matches(&self, element: &StyleSignature) -> bool {
        if self.level != -1 {
            if element.level == -1 {
                return false;
            }
            let level_fits = if self.starred {
                element.level >= self.level
            } else {
                element.level == self.level
            };
            if !level_fits {
                return false;
            }
        }
        if element.starred && !self.starred {
            return false;
        }
        element.names.contains_all(self.names) && element.stereotypes.is_superset(&self.stereotypes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rule_applies_to_elements_with_at_least_its_names() {
        let rule = StyleSignature::of(&[SName::Root, SName::Document]);
        let element = StyleSignature::of(&[SName::Root, SName::Document, SName::Title]);
        assert!(rule.matches(&element));
        assert!(!element.matches(&rule));
    }

    #[test]
    fn stereotypes_must_all_be_present() {
        let rule = StyleSignature::of(&[SName::Node]).with_stereotype(".Big_One");
        let element = StyleSignature::of(&[SName::Node, SName::Root]).with_stereotype("bigone");
        assert!(rule.matches(&element));
        assert!(!rule.matches(&StyleSignature::of(&[SName::Node])));
    }

    #[test]
    fn depth_rules_need_the_depth_or_deeper_when_starred() {
        let at_two = StyleSignature::of(&[SName::Node]).with_level(2);
        let element = |level| StyleSignature::of(&[SName::Node]).with_level(level);
        assert!(at_two.matches(&element(2)));
        assert!(!at_two.matches(&element(3)));
        assert!(at_two.with_star().matches(&element(3)));
        assert!(!at_two.matches(&StyleSignature::of(&[SName::Node])));
        assert!(!at_two.matches(&element(2).with_star()));
    }
}
