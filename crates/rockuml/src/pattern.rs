//! Compiles PlantUML's Java regular expressions with the `regex` crate without changing their meaning.
//!
//! Java's `\s`, `\w`, `\d` and `\b` are ASCII-only while the `regex` crate's are Unicode-aware, `\<` and `\>`
//! are plain characters in Java but word-boundary assertions here, and `\uXXXX` is spelled `\x{XXXX}`.

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
    Regex::new(&format!("{flags}{}", translate(pattern)?)).ok()
}

fn expand_macros(pattern: &str) -> String {
    pattern
        .replace("%pLN", r"\p{L}\p{N}")
        .replace("%s", "\\s\u{A0}")
        .replace("%q", "'\u{2018}\u{2019}")
        .replace("%g", "\"\u{201C}\u{201D}\u{E121}")
}

const JAVA_SPACES: &str = r"\t\n\x0B\x0C\r ";
const JAVA_WORD: &str = "a-zA-Z0-9_";
const JAVA_DIGITS: &str = "0-9";

fn translate(pattern: &str) -> Option<String> {
    let mut result = String::with_capacity(pattern.len() * 2);
    let mut chars = pattern.chars().peekable();
    let mut class_depth = 0;
    while let Some(c) = chars.next() {
        match c {
            '\\' => translate_escape(chars.next()?, class_depth > 0, &mut chars, &mut result)?,
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
        'b' if !in_class => result.push_str(r"(?-u:\b)"),
        'B' if !in_class => result.push_str(r"(?-u:\B)"),
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
}
