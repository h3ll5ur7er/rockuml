//! Recognises the `@start...`, `@end...`, `@pause`/`@unpause` and `!exit` directives that frame diagram blocks.

use std::sync::LazyLock;

use regex::Regex;

use crate::java;
use crate::text::StringLocated;

pub fn is_start_directive(s: &str) -> bool {
    let rest = s.trim_start_matches(java::is_whitespace);
    let Some(after_marker) = rest.strip_prefix(['@', '\\']) else {
        return false;
    };
    after_marker.starts_with("start") && after_marker.chars().count() > 5
}

pub fn is_end_directive(s: &str) -> bool {
    starts_with_directive_keyword(s, "end")
}

pub fn is_pause_directive(s: &str) -> bool {
    starts_with_directive_keyword(s, "pause")
}

pub fn is_unpause_directive(s: &str) -> bool {
    starts_with_directive_keyword(s, "unpause")
}

pub fn is_exit(s: &str) -> bool {
    s.trim_matches(java::is_whitespace) == "!exit"
}

/// Text that precedes `@start` on its line, as long as it holds no word characters: a comment
/// prefix such as `' ` that every line of the block carries and that must be stripped.
pub fn before_start_uml(s: &str) -> Option<&str> {
    let mut inside_angle_brackets = false;
    for (index, c) in s.char_indices() {
        if starts_with_directive_keyword(&s[index..], "start") {
            return Some(&s[..index]);
        }
        if inside_angle_brackets {
            if c == '>' {
                inside_angle_brackets = false;
            }
            continue;
        }
        if c == '<' {
            inside_angle_brackets = true;
        } else if c == '~' || java::is_letter_or_digit(c) || c == '_' {
            return None;
        }
    }
    None
}

fn starts_with_directive_keyword(text: &str, keyword: &str) -> bool {
    let rest = text.trim_start_matches(java::is_whitespace);
    rest.strip_prefix(['@', '\\'])
        .is_some_and(|after_marker| after_marker.starts_with(keyword))
}

/// While paused, a line `@append text` (or `@a text`) still contributes `text` to the block.
pub fn possible_append(line: &StringLocated) -> Option<StringLocated> {
    static APPEND: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)^[^a-zA-Z0-9_]*[@\\](append|a)(?-u:\b)").unwrap());
    let matched = APPEND.find(line.text())?;
    Some(line.with_text(java::trim(&line.text()[matched.end()..])))
}

/// `@startditaa` blocks keep their trailing backslashes: they are part of the drawing.
pub fn is_ditaa_start(s: &str) -> bool {
    let rest = s.trim_start_matches(java::is_whitespace);
    let Some(after_marker) = rest.strip_prefix(['@', '\\']) else {
        return false;
    };
    after_marker.to_ascii_lowercase().starts_with("startditaa")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::LineLocation;

    #[test]
    fn start_directive_needs_something_after_start() {
        assert!(is_start_directive("  @startuml"));
        assert!(is_start_directive("\\startmindmap"));
        assert!(!is_start_directive("@start"));
        assert!(!is_start_directive("' @startuml"));
    }

    #[test]
    fn exit_must_be_alone_on_its_line() {
        assert!(is_exit("  !exit "));
        assert!(!is_exit("!exit now"));
    }

    #[test]
    fn prefix_before_start_is_kept_only_without_word_characters() {
        assert_eq!(before_start_uml("' @startuml"), Some("'"));
        assert_eq!(before_start_uml("<b>@startuml"), Some("<b>"));
        assert_eq!(before_start_uml("x @startuml"), None);
        assert_eq!(before_start_uml("Alice -> Bob"), None);
    }

    #[test]
    fn append_lines_keep_their_text() {
        let line = StringLocated::new("' @append  more ", LineLocation::new("t", None));
        assert_eq!(possible_append(&line).unwrap().text(), "more");
        let line = StringLocated::new("@apple", LineLocation::new("t", None));
        assert!(possible_append(&line).is_none());
    }

    #[test]
    fn ditaa_is_detected_case_insensitively() {
        assert!(is_ditaa_start("@StartDitaa(scale=2)"));
        assert!(!is_ditaa_start("@startdot"));
        assert!(!is_ditaa_start("@startuml"));
    }
}
