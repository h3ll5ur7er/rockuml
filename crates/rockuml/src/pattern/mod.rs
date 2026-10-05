//! Compiles PlantUML's Java regular expressions without changing their meaning.
//!
//! Java's `\s`, `\w`, `\d` and `\b` are ASCII-only while the `regex` crate's are Unicode-aware, `\<` and `\>`
//! are plain characters in Java but word-boundary assertions here, and `\uXXXX` is spelled `\x{XXXX}`.
//! Patterns with lookarounds go to `fancy-regex`, which cannot switch Unicode mode inline, so there the
//! ASCII word boundary is spelled out with lookarounds instead.

mod tree;

pub use tree::{RegexResult, RegexTree};

use regex::Regex;

/// Compiles like PlantUML's `Pattern2`: `%s`, `%q`, `%g` and `%pLN` macros expanded, case-insensitive.
pub fn plantuml_regex(pattern: &str) -> Regex {
    java_regex(&expand_macros(pattern), true)
}

pub fn java_regex(pattern: &str, case_insensitive: bool) -> Regex {
    try_java_regex(pattern, case_insensitive)
        .unwrap_or_else(|| panic!("cannot translate Java regex {pattern:?}"))
}

/// For patterns written by users, which may be invalid or use Java syntax with no equivalent here.
pub fn try_java_regex(pattern: &str, case_insensitive: bool) -> Option<Regex> {
    let flags = if case_insensitive { "(?i)" } else { "" };
    Regex::new(&format!("{flags}{}", translate(pattern, Dialect::Regex)?)).ok()
}

/// A Java pattern compiled with whichever engine supports its syntax.
#[derive(Debug)]
pub enum JavaPattern {
    Plain(Regex),
    WithLookaround(fancy_regex::Regex),
}

impl JavaPattern {
    /// Compiles like PlantUML's `Pattern2` (macros expanded, case-insensitive).
    pub fn plantuml(pattern: &str) -> Self {
        let expanded = expand_macros(pattern);
        if has_lookaround(&expanded) {
            let translated = translate(&expanded, Dialect::FancyRegex)
                .unwrap_or_else(|| panic!("cannot translate Java regex {pattern:?}"));
            let compiled = fancy_regex::Regex::new(&format!("(?i){translated}"))
                .unwrap_or_else(|error| panic!("cannot compile Java regex {pattern:?}: {error}"));
            JavaPattern::WithLookaround(compiled)
        } else {
            JavaPattern::Plain(java_regex(&expanded, true))
        }
    }

    pub fn is_match(&self, text: &str) -> bool {
        match self {
            JavaPattern::Plain(regex) => regex.is_match(text),
            JavaPattern::WithLookaround(regex) => regex.is_match(text).unwrap_or(false),
        }
    }

    /// The groups of the first match, numbered from 1 as in Java; `None` for groups that did not take part.
    pub fn captures(&self, text: &str) -> Option<Vec<Option<String>>> {
        let to_vec = |count: usize, group: &dyn Fn(usize) -> Option<String>| (1..count).map(group).collect();
        match self {
            JavaPattern::Plain(regex) => {
                let captures = regex.captures(text)?;
                Some(to_vec(captures.len(), &|index| captures.get(index).map(|group| group.as_str().to_owned())))
            }
            JavaPattern::WithLookaround(regex) => {
                let captures = regex.captures(text).ok()??;
                Some(to_vec(captures.len(), &|index| captures.get(index).map(|group| group.as_str().to_owned())))
            }
        }
    }
}

fn has_lookaround(pattern: &str) -> bool {
    ["(?=", "(?!", "(?<=", "(?<!"].iter().any(|syntax| pattern.contains(syntax))
}

fn expand_macros(pattern: &str) -> String {
    pattern
        .replace("%pLN", r"\p{L}\p{N}")
        .replace("%s", "\\s\u{A0}")
        .replace("%q", "'\u{2018}\u{2019}")
        .replace("%g", "\"\u{201C}\u{201D}\u{E121}")
}

#[derive(Clone, Copy, PartialEq)]
enum Dialect {
    Regex,
    FancyRegex,
}

const JAVA_SPACES: &str = r"\t\n\x0B\x0C\r ";
const JAVA_WORD: &str = "a-zA-Z0-9_";
const JAVA_DIGITS: &str = "0-9";
const FANCY_WORD_BOUNDARY: &str = r"(?:(?<=[a-zA-Z0-9_])(?![a-zA-Z0-9_])|(?<![a-zA-Z0-9_])(?=[a-zA-Z0-9_]))";
const FANCY_NOT_WORD_BOUNDARY: &str = r"(?:(?<=[a-zA-Z0-9_])(?=[a-zA-Z0-9_])|(?<![a-zA-Z0-9_])(?![a-zA-Z0-9_]))";

fn translate(pattern: &str, dialect: Dialect) -> Option<String> {
    let mut result = String::with_capacity(pattern.len() * 2);
    let mut chars = pattern.chars().peekable();
    let mut class_depth = 0;
    while let Some(c) = chars.next() {
        match c {
            '\\' => translate_escape(chars.next()?, class_depth > 0, dialect, &mut chars, &mut result)?,
            '[' => {
                class_depth += 1;
                result.push(c);
                if chars.peek() == Some(&'^') {
                    result.push(chars.next()?);
                }
                if chars.peek() == Some(&']') {
                    result.push_str(r"\]");
                    chars.next();
                }
            }
            ']' if class_depth > 0 => {
                class_depth -= 1;
                result.push(c);
            }
            // The regex crate reads `&&`, `~~` and `--` inside classes as set operations; Java does not.
            '&' | '~' if class_depth > 0 => {
                result.push('\\');
                result.push(c);
            }
            '-' if class_depth > 0 && chars.peek() == Some(&'-') => result.push_str(r"\-"),
            _ => result.push(c),
        }
    }
    Some(result)
}

fn translate_escape(
    escaped: char,
    in_class: bool,
    dialect: Dialect,
    chars: &mut std::iter::Peekable<std::str::Chars>,
    result: &mut String,
) -> Option<()> {
    let class = |members: &str, negated: bool| match (in_class, negated) {
        (true, false) => Some(members.to_owned()),
        (false, false) => Some(format!("[{members}]")),
        (false, true) => Some(format!("[^{members}]")),
        (true, true) => None,
    };
    match escaped {
        's' => result.push_str(&class(JAVA_SPACES, false)?),
        'S' => result.push_str(&class(JAVA_SPACES, true)?),
        'w' => result.push_str(&class(JAVA_WORD, false)?),
        'W' => result.push_str(&class(JAVA_WORD, true)?),
        'd' => result.push_str(&class(JAVA_DIGITS, false)?),
        'D' => result.push_str(&class(JAVA_DIGITS, true)?),
        'b' if !in_class && dialect == Dialect::Regex => result.push_str(r"(?-u:\b)"),
        'B' if !in_class && dialect == Dialect::Regex => result.push_str(r"(?-u:\B)"),
        'b' if !in_class => result.push_str(FANCY_WORD_BOUNDARY),
        'B' if !in_class => result.push_str(FANCY_NOT_WORD_BOUNDARY),
        'u' => {
            result.push_str(r"\x{");
            result.extend(chars.by_ref().take(4));
            result.push('}');
        }
        'Q' => {
            let mut quoted = String::new();
            while let Some(c) = chars.next() {
                if c == '\\' && chars.peek() == Some(&'E') {
                    chars.next();
                    break;
                }
                quoted.push(c);
            }
            result.push_str(&regex::escape(&quoted));
        }
        '<' | '>' => result.push(escaped),
        c if c.is_ascii() => {
            result.push('\\');
            result.push(c);
        }
        c => result.push(c),
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shorthand_classes_are_ascii_only_like_java() {
        let word = java_regex(r"^\w+$", false);
        assert!(word.is_match("abc_09"));
        assert!(!word.is_match("é"));
        assert!(!java_regex(r"^\s$", false).is_match("\u{2003}"));
    }

    #[test]
    fn shorthands_inside_classes_expand_in_place() {
        let regex = plantuml_regex("^[%s{]+$");
        assert!(regex.is_match(" \u{A0}{\t"));
        assert!(!regex.is_match("x"));
    }

    #[test]
    fn angle_brackets_are_literal() {
        assert!(java_regex(r"\<b\>", false).is_match("<b>"));
    }

    #[test]
    fn unicode_escapes_and_quoting_are_translated() {
        assert!(java_regex(r"^\u00e9$", false).is_match("é"));
        assert!(java_regex(r"^\Q%a.b%\E$", false).is_match("%a.b%"));
    }

    #[test]
    fn word_boundary_ignores_non_ascii_letters_like_java() {
        assert!(java_regex(r"x\b", false).is_match("xé"));
        assert!(JavaPattern::plantuml(r"x\b(?!y)").is_match("xé"));
        assert!(!JavaPattern::plantuml(r"x\b(?!y)").is_match("xa"));
    }

    #[test]
    fn invalid_patterns_are_rejected_instead_of_panicking() {
        assert!(try_java_regex(r"a\", false).is_none());
        assert!(try_java_regex("(", false).is_none());
    }

    #[test]
    fn plantuml_patterns_are_case_insensitive() {
        assert!(plantuml_regex("^@startuml").is_match("@StartUML"));
    }

    #[test]
    fn lookbehind_patterns_use_the_backtracking_engine() {
        let grouping = JavaPattern::plantuml(r"^group((?<!else)(?<!also)(?<!end)#\w+)?$");
        assert!(matches!(grouping, JavaPattern::WithLookaround(_)));
        assert_eq!(grouping.captures("group#red"), Some(vec![Some("#red".to_owned())]));
    }
}
