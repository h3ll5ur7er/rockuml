//! PlantUML's composable command patterns (`RegexLeaf`, `RegexConcat`, `RegexOr`, ...).
//!
//! The pieces are concatenated into one Java regex; leaves declare how many capturing groups they hold so
//! the positional groups of a match can be handed back under the leaves' names.

use std::sync::OnceLock;

use super::JavaPattern;
use crate::java::JavaHashMap;

/// The values a name captured; `None` where its group did not take part in the match.
type Captured = Vec<Option<String>>;

#[derive(Debug)]
pub enum RegexTree {
    Leaf {
        name: Option<&'static str>,
        group_count: usize,
        pattern: &'static str,
    },
    Concat(Vec<RegexTree>, OnceLock<JavaPattern>),
    Or {
        name: Option<&'static str>,
        alternatives: Vec<RegexTree>,
    },
    Optional(Box<RegexTree>),
    /// PlantUML always writes this as a capturing group but only reads it when named, so an unnamed one
    /// shifts every later group: a quirk kept as is.
    OneOrMore {
        name: Option<&'static str>,
        part: Box<RegexTree>,
    },
    ZeroOrMore(Box<RegexTree>),
}

impl RegexTree {
    pub fn leaf(pattern: &'static str) -> Self {
        Self::Leaf {
            name: None,
            group_count: 0,
            pattern,
        }
    }

    pub fn groups(group_count: usize, pattern: &'static str) -> Self {
        Self::Leaf {
            name: None,
            group_count,
            pattern,
        }
    }

    pub fn named(group_count: usize, name: &'static str, pattern: &'static str) -> Self {
        Self::Leaf {
            name: Some(name),
            group_count,
            pattern,
        }
    }

    pub fn start() -> Self {
        Self::leaf("^")
    }

    pub fn end() -> Self {
        Self::leaf("$")
    }

    pub fn spaces_zero_or_more() -> Self {
        Self::leaf("[%s]*")
    }

    pub fn spaces_one_or_more() -> Self {
        Self::leaf("[%s]+")
    }

    pub fn space_one() -> Self {
        Self::leaf("[%s]")
    }

    pub fn space_zero_or_one() -> Self {
        Self::optional(Self::space_one())
    }

    pub fn concat(parts: Vec<RegexTree>) -> Self {
        Self::Concat(parts, OnceLock::new())
    }

    pub fn or(alternatives: Vec<RegexTree>) -> Self {
        Self::Or {
            name: None,
            alternatives,
        }
    }

    pub fn named_or(name: &'static str, alternatives: Vec<RegexTree>) -> Self {
        Self::Or {
            name: Some(name),
            alternatives,
        }
    }

    pub fn optional(part: RegexTree) -> Self {
        Self::Optional(Box::new(part))
    }

    pub fn one_or_more(name: Option<&'static str>, part: RegexTree) -> Self {
        Self::OneOrMore {
            name,
            part: Box::new(part),
        }
    }

    pub fn zero_or_more(part: RegexTree) -> Self {
        Self::ZeroOrMore(Box::new(part))
    }

    pub fn pattern_string(&self) -> String {
        match self {
            Self::Leaf { pattern, .. } => (*pattern).to_owned(),
            Self::Concat(parts, _) => parts.iter().map(Self::pattern_string).collect(),
            Self::Or { name, alternatives } => {
                let alternatives: Vec<String> = alternatives.iter().map(Self::pattern_string).collect();
                let capture = if name.is_some() { "" } else { "?:" };
                format!("({capture}{})", alternatives.join("|"))
            }
            Self::Optional(part) => format!("(?:{})?", part.pattern_string()),
            Self::OneOrMore { part, .. } => format!("({})+", part.pattern_string()),
            Self::ZeroOrMore(part) => format!("(?:{})*", part.pattern_string()),
        }
    }

    /// Whether the line matches anywhere (PlantUML's `find`). Only concatenations are matched directly.
    pub fn is_match(&self, line: &str) -> bool {
        self.compiled().is_match(line)
    }

    /// The named groups of the first match.
    pub fn matcher(&self, line: &str) -> Option<RegexResult> {
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
    fn create_partial_match(&self, groups: &mut impl Iterator<Item = Option<String>>) -> JavaHashMap<Captured> {
        match self {
            Self::Leaf { name, group_count, .. } => {
                let captured: Captured = groups.by_ref().take(*group_count).collect();
                let mut result = JavaHashMap::default();
                if let Some(name) = name {
                    result.put((*name).to_owned(), captured);
                }
                result
            }
            Self::Concat(parts, _) => Self::composed_partial_match(parts.iter(), groups),
            Self::Optional(part) | Self::ZeroOrMore(part) => Self::composed_partial_match([&**part], groups),
            Self::Or { name, alternatives } => Self::named_group_partial_match(*name, alternatives.iter(), groups),
            Self::OneOrMore { name, part } => Self::named_group_partial_match(*name, [&**part], groups),
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

    fn named_group_partial_match<'a>(
        name: Option<&'static str>,
        parts: impl IntoIterator<Item = &'a RegexTree>,
        groups: &mut impl Iterator<Item = Option<String>>,
    ) -> JavaHashMap<Captured> {
        let whole = name.map(|_| groups.next().flatten());
        let mut result = JavaHashMap::default();
        result.put_all(Self::composed_partial_match(parts, groups));
        if let (Some(name), Some(whole)) = (name, whole) {
            result.put(name.to_owned(), vec![whole]);
        }
        result
    }
}

/// The groups of a match, by the names the command's pattern gave them.
#[derive(Debug)]
pub struct RegexResult {
    data: JavaHashMap<Captured>,
}

impl RegexResult {
    pub fn get(&self, name: &str, index: usize) -> Option<&str> {
        self.data.get(name)?.get(index)?.as_deref()
    }

    /// The first group whose name starts with `prefix` and that matched, in Java's `HashMap` order.
    pub fn get_lazzy(&self, prefix: &str, index: usize) -> Option<&str> {
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
            RegexTree::named_or(
                "ARROW",
                vec![RegexTree::named(1, "SOLID", "(-+>)"), RegexTree::named(1, "DOTTED", "(\\.+>)")],
            ),
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
        assert_eq!(result.get("ARROW", 0), Some("..>"));
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
    fn unnamed_repetitions_capture_without_being_read_like_plantuml() {
        let tree = RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::one_or_more(None, RegexTree::leaf("[ab]")),
            RegexTree::named(1, "REST", "(.*)"),
            RegexTree::end(),
        ]);
        assert_eq!(tree.pattern_string(), "^([ab])+(.*)$");
        assert_eq!(tree.matcher("abX").unwrap().get("REST", 0), Some("b"));
    }

    #[test]
    fn patterns_are_matched_case_insensitively_anywhere() {
        assert!(arrow().is_match("A -> B"));
        assert!(arrow().matcher("A => B").is_none());
    }
}
