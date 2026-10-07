//! Source lines that remember where they came from, so errors can point at the right file and line.

use std::rc::Rc;
use std::sync::LazyLock;

use regex::Regex;

use crate::java;

#[derive(Clone, Debug, PartialEq)]
pub struct LineLocation {
    description: Rc<str>,
    /// Zero-based; -1 before the first line has been read.
    position: i32,
    parent: Option<Rc<LineLocation>>,
}

impl LineLocation {
    pub fn new(description: &str, parent: Option<LineLocation>) -> Self {
        Self {
            description: description.into(),
            position: -1,
            parent: parent.map(Rc::new),
        }
    }

    #[must_use]
    pub fn one_line_read(&self) -> Self {
        Self {
            position: self.position + 1,
            ..self.clone()
        }
    }

    pub fn position(&self) -> i32 {
        self.position
    }

    /// The file or resource the line comes from.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Where the file was included from.
    pub fn parent(&self) -> Option<&LineLocation> {
        self.parent.as_deref()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StringLocated {
    text: String,
    location: LineLocation,
    preprocessor_error: Option<String>,
}

impl StringLocated {
    pub fn new(text: impl Into<String>, location: LineLocation) -> Self {
        Self {
            text: text.into(),
            location,
            preprocessor_error: None,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn location(&self) -> &LineLocation {
        &self.location
    }

    #[must_use]
    pub fn with_preprocessor_error(&self, error: impl Into<String>) -> Self {
        Self {
            preprocessor_error: Some(error.into()),
            ..self.clone()
        }
    }

    #[must_use]
    pub fn with_text(&self, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..self.clone()
        }
    }

    #[must_use]
    pub fn append(&self, end_of_line: &str) -> Self {
        self.with_text(self.text.clone() + end_of_line)
    }

    #[must_use]
    pub fn trimmed(&self) -> Self {
        self.with_text(java::trim(&self.text))
    }

    /// Joins a line ending with a single backslash to the next one, as a continuation.
    #[must_use]
    pub fn merge_end_backslash(&self, next: &StringLocated) -> Self {
        debug_assert!(ends_with_backslash(&self.text));
        let without_backslash = &self.text[..self.text.len() - 1];
        self.with_text(format!("{without_backslash}{}", next.text))
    }

    /// Strips `/' ... '/` comments that share the line with content.
    #[must_use]
    pub fn remove_inner_comment(&self) -> Self {
        let trimmed = self.text.replace('\t', " ");
        let trimmed = trimmed.trim_matches(|c: char| c <= ' ');
        if trimmed.starts_with("/'")
            && let Some(index) = self.text.find("'/")
        {
            return self.with_text(remove_special_inner_comment(&self.text[index + 2..]));
        }
        if trimmed.ends_with("'/")
            && let Some(index) = self.text.rfind("/'")
        {
            return self.with_text(remove_special_inner_comment(&self.text[..index]));
        }
        if trimmed.contains("/'''") && trimmed.contains("'''/") {
            return self.with_text(remove_special_inner_comment(&self.text));
        }
        self.clone()
    }
}

/// A name written between any of PlantUML's double quotes loses them
/// (`eventuallyRemoveStartingAndEndingDoubleQuote`).
pub(crate) fn unquoted(text: &str) -> &str {
    let is_double_quote = |c| matches!(c, '"' | '\u{201C}' | '\u{201D}' | '\u{AB}' | '\u{BB}');
    let mut chars = text.chars();
    match (chars.next(), chars.next_back()) {
        (Some(first), Some(last)) if is_double_quote(first) && is_double_quote(last) => {
            &text[first.len_utf8()..text.len() - last.len_utf8()]
        }
        _ => text,
    }
}

/// Where an arrow points on the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
    Right,
    Left,
    Down,
    Up,
}

impl Direction {
    /// The direction an arrow's body asks for, like `left`, `ri` or `u`; by default down, or right for a
    /// one-character body (`StringUtils.getQueueDirection`).
    pub(crate) fn of_queue(queue: &str) -> Self {
        let queue = queue.to_lowercase();
        let named = [
            ("left", Self::Left),
            ("right", Self::Right),
            ("up", Self::Up),
            ("down", Self::Down),
            ("l", Self::Left),
            ("r", Self::Right),
            ("u", Self::Up),
            ("d", Self::Down),
        ];
        named
            .into_iter()
            .find(|(name, _)| queue.contains(name))
            .map_or_else(
                || {
                    if queue.chars().count() == 1 {
                        Self::Right
                    } else {
                        Self::Down
                    }
                },
                |(_, direction)| direction,
            )
    }
}

pub(crate) fn ends_with_backslash(s: &str) -> bool {
    s.ends_with('\\') && !s.ends_with("\\\\")
}

fn remove_special_inner_comment(s: &str) -> String {
    static SPECIAL_INNER_COMMENT: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"/'''[-A-Za-z0-9_]*'''/").unwrap());

    if s.contains("/'''") && s.contains("'''/") {
        SPECIAL_INNER_COMMENT.replace_all(s, "").into_owned()
    } else {
        s.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str) -> StringLocated {
        StringLocated::new(text, LineLocation::new("test", None))
    }

    #[test]
    fn locations_count_lines_from_zero() {
        let location = LineLocation::new("file", None);
        assert_eq!(location.one_line_read().one_line_read().position(), 1);
    }

    #[test]
    fn a_trailing_double_backslash_is_not_a_continuation() {
        assert!(ends_with_backslash("a \\"));
        assert!(!ends_with_backslash("a \\\\"));
    }

    #[test]
    fn inner_comments_are_removed_from_either_end() {
        assert_eq!(
            line("/' lead '/ Alice -> Bob")
                .remove_inner_comment()
                .text(),
            " Alice -> Bob"
        );
        assert_eq!(
            line("Alice -> Bob /' tail '/")
                .remove_inner_comment()
                .text(),
            "Alice -> Bob "
        );
        assert_eq!(line("A /'''x'''/ B").remove_inner_comment().text(), "A  B");
        assert_eq!(line("A -> B").remove_inner_comment().text(), "A -> B");
    }
}
