//! PlantUML's composable command patterns (`RegexLeaf`, `RegexConcat`, `RegexOr`, ...).
//!
//! The pieces are concatenated into one Java regex; leaves declare how many capturing groups they hold so
//! the positional groups of a match can be handed back under the leaves' names.

use std::borrow::Cow;
use std::sync::OnceLock;

use super::JavaPattern;
use crate::java::JavaHashMap;

/// The values a name captured; `None` where its group did not take part in the match.
type Captured = Vec<Option<String>>;

#[derive(Debug)]
pub(crate) enum RegexTree {
    Leaf {
        name: Option<&'static str>,
        group_count: usize,
        pattern: Cow<'static, str>,
    },
    Concat(Vec<RegexTree>, OnceLock<JavaPattern>),
    /// A named alternation also captures what matched as a whole.
    Or(Option<&'static str>, Vec<RegexTree>),
    Optional(Box<RegexTree>),
}

impl RegexTree {
    pub(crate) fn leaf(pattern: impl Into<Cow<'static, str>>) -> Self {
        Self::Leaf {
            name: None,
            group_count: 0,
            pattern: pattern.into(),
        }
    }

    pub(crate) fn named(
        group_count: usize,
        name: &'static str,
        pattern: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::Leaf {
            name: Some(name),
            group_count,
            pattern: pattern.into(),
        }
    }

    pub(crate) fn start() -> Self {
        Self::leaf("^")
    }

    pub(crate) fn end() -> Self {
        Self::leaf("$")
    }

    pub(crate) fn spaces_zero_or_more() -> Self {
        Self::leaf("[%s]*")
    }

    pub(crate) fn spaces_one_or_more() -> Self {
        Self::leaf("[%s]+")
    }

    pub(crate) fn concat(parts: Vec<RegexTree>) -> Self {
        Self::Concat(parts, OnceLock::new())
    }

    pub(crate) fn or(alternatives: Vec<RegexTree>) -> Self {
        Self::Or(None, alternatives)
    }

    pub(crate) fn named_or(name: &'static str, alternatives: Vec<RegexTree>) -> Self {
        Self::Or(Some(name), alternatives)
    }

    pub(crate) fn optional(part: RegexTree) -> Self {
        Self::Optional(Box::new(part))
    }

    /// The Java regex the tree stands for, in PlantUML's dialect.
    pub(crate) fn pattern_string(&self) -> String {
        match self {
            Self::Leaf { pattern, .. } => pattern.to_string(),
            Self::Concat(parts, _) => parts.iter().map(Self::pattern_string).collect(),
            Self::Or(name, alternatives) => {
                let alternatives: Vec<String> =
                    alternatives.iter().map(Self::pattern_string).collect();
                let group = if name.is_some() { "" } else { "?:" };
                format!("({group}{})", alternatives.join("|"))
            }
            Self::Optional(part) => format!("(?:{})?", part.pattern_string()),
        }
    }

    /// Whether the line matches anywhere (PlantUML's `find`). Only concatenations are matched directly.
    pub(crate) fn is_match(&self, line: &str) -> bool {
        self.compiled().is_match(line)
    }

    /// The named groups of the first match.
    pub(crate) fn matcher(&self, line: &str) -> Option<RegexResult> {
        let groups = self.compiled().captures(line)?;
        let mut groups = groups.into_iter();
        Some(RegexResult {
            data: self.create_partial_match(&mut groups),
        })
    }

    fn compiled(&self) -> &JavaPattern {
        let Self::Concat(_, compiled) = self else {
            panic!("only a concatenation can be matched directly, as in PlantUML");
        };
        compiled.get_or_init(|| JavaPattern::plantuml(&self.pattern_string()))
    }

    /// Maps the next groups to names, building maps in the same order PlantUML does so that iterating the
    /// result follows Java's `HashMap` order.
    fn create_partial_match(
        &self,
        groups: &mut impl Iterator<Item = Option<String>>,
    ) -> JavaHashMap<Captured> {
        match self {
            Self::Leaf {
                name, group_count, ..
            } => {
                let captured: Captured = groups.by_ref().take(*group_count).collect();
                let mut result = JavaHashMap::default();
                if let Some(name) = name {
                    result.put((*name).to_owned(), captured);
                }
                result
            }
            Self::Concat(parts, _) | Self::Or(None, parts) => {
                Self::composed_partial_match(parts.iter(), groups)
            }
            Self::Or(Some(name), parts) => {
                let whole = groups.next().flatten();
                let mut result = Self::composed_partial_match(parts.iter(), groups);
                result.put((*name).to_owned(), vec![whole]);
                result
            }
            Self::Optional(part) => Self::composed_partial_match([&**part], groups),
        }
    }

    fn composed_partial_match<'a>(
        parts: impl IntoIterator<Item = &'a RegexTree>,
        groups: &mut impl Iterator<Item = Option<String>>,
    ) -> JavaHashMap<Captured> {
        let mut result = JavaHashMap::default();
        for part in parts {
            result.put_all(part.create_partial_match(groups));
        }
        result
    }
}

/// The groups of a match, by the names the command's pattern gave them.
#[derive(Debug)]
pub(crate) struct RegexResult {
    data: JavaHashMap<Captured>,
}

impl RegexResult {
    pub(crate) fn get(&self, name: &str, index: usize) -> Option<&str> {
        self.data.get(name)?.get(index)?.as_deref()
    }

    /// The first group whose name starts with `prefix` and that matched, in Java's `HashMap` order.
    pub(crate) fn get_lazzy(&self, prefix: &str, index: usize) -> Option<&str> {
        self.data
            .iter()
            .filter(|(name, _)| name.starts_with(prefix))
            .find_map(|(_, captured)| captured.get(index)?.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arrow() -> RegexTree {
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "FROM", "([%pLN_]+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::or(vec![
                RegexTree::named(1, "SOLID", "(-+>)"),
                RegexTree::named(1, "DOTTED", "(\\.+>)"),
            ]),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "TO", "([%pLN_]+)"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_zero_or_more(),
                RegexTree::leaf(":"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "LABEL", "(.*)"),
            ])),
            RegexTree::end(),
        ])
    }

    #[test]
    fn named_groups_are_recovered_from_positions() {
        let result = arrow().matcher("Alice ..> Bob : hi").unwrap();
        assert_eq!(result.get("FROM", 0), Some("Alice"));
        assert_eq!(result.get("SOLID", 0), None);
        assert_eq!(result.get("DOTTED", 0), Some("..>"));
        assert_eq!(result.get("TO", 0), Some("Bob"));
        assert_eq!(result.get("LABEL", 0), Some("hi"));
    }

    #[test]
    fn lazy_lookup_finds_the_alternative_that_matched() {
        let result = arrow().matcher("a->b").unwrap();
        assert_eq!(result.get_lazzy("SOL", 0), Some("->"));
        assert_eq!(result.get_lazzy("DOT", 0), None);
        assert_eq!(result.get("LABEL", 0), None);
    }

    #[test]
    fn patterns_are_matched_case_insensitively_anywhere() {
        assert!(arrow().is_match("A -> B"));
        assert!(arrow().matcher("A => B").is_none());
    }
}
