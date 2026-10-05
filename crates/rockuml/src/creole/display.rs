use crate::jaws::{
    BLOCK_E1_BREAKLINE, BLOCK_E1_NEWLINE, BLOCK_E1_NEWLINE_LEFT_ALIGN,
    BLOCK_E1_NEWLINE_RIGHT_ALIGN, BLOCK_E1_REAL_BACKSLASH,
};
use crate::klimt::HorizontalAlignment;

/// Marks a quote PlantUML keeps out of the text.
const BLOCK_E1_INVISIBLE_QUOTE: char = '\u{E121}';

/// The lines of a label, with the alignment its line breaks asked for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Display {
    lines: Vec<String>,
    natural_alignment: Option<HorizontalAlignment>,
}

impl Display {
    pub fn create(lines: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            lines: lines.into_iter().map(Into::into).collect(),
            natural_alignment: None,
        }
    }

    /// A label written on one line: `\n`, `\l` and `\r` break it (left- or right-aligning it for the last
    /// two), except inside `<math>`, `<latex>` and `[[links]]`.
    pub fn with_newlines(text: &str) -> Self {
        let mut lines = Vec::new();
        let mut current = String::new();
        let mut natural_alignment = None;
        let mut raw = false;
        let chars: Vec<char> = text.chars().collect();
        let mut index = 0;
        while index < chars.len() {
            let c = chars[index];
            let rest: String = chars[index..].iter().take(8).collect();
            if ["<math>", "<latex>", "[["]
                .iter()
                .any(|start| rest.starts_with(start))
            {
                raw = true;
            } else if ["</math>", "</latex>", "]]"]
                .iter()
                .any(|end| rest.starts_with(end))
            {
                raw = false;
            }
            let mut break_line = |alignment: Option<HorizontalAlignment>, current: &mut String| {
                if alignment.is_some() {
                    natural_alignment = alignment;
                }
                lines.push(std::mem::take(current));
            };
            match c {
                '\\' if !raw && index + 1 < chars.len() => {
                    index += 1;
                    match chars[index] {
                        'n' => break_line(None, &mut current),
                        'r' => break_line(Some(HorizontalAlignment::Right), &mut current),
                        'l' => break_line(Some(HorizontalAlignment::Left), &mut current),
                        't' => current.push('\t'),
                        '\\' => current.push('\\'),
                        other => {
                            current.push('\\');
                            current.push(other);
                        }
                    }
                }
                BLOCK_E1_REAL_BACKSLASH => current.push('\\'),
                BLOCK_E1_NEWLINE_LEFT_ALIGN => {
                    break_line(Some(HorizontalAlignment::Left), &mut current);
                }
                BLOCK_E1_NEWLINE_RIGHT_ALIGN => {
                    break_line(Some(HorizontalAlignment::Right), &mut current);
                }
                BLOCK_E1_INVISIBLE_QUOTE => {}
                BLOCK_E1_NEWLINE if !raw => break_line(None, &mut current),
                BLOCK_E1_BREAKLINE => break_line(None, &mut current),
                _ => current.push(c),
            }
            index += 1;
        }
        lines.push(current);
        Self {
            lines,
            natural_alignment,
        }
    }

    /// Written `\t` becomes a tabulation.
    #[must_use]
    pub fn replace_backslash_t(&self) -> Self {
        Self {
            lines: self
                .lines
                .iter()
                .map(|line| line.replace("\\t", "\t"))
                .collect(),
            natural_alignment: self.natural_alignment,
        }
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn natural_alignment(&self) -> Option<HorizontalAlignment> {
        self.natural_alignment
    }

    /// No lines, or a single one of only (ASCII) whitespace.
    pub fn is_white(&self) -> bool {
        match self.lines.as_slice() {
            [] => true,
            [only] => only
                .chars()
                .all(|c| matches!(c, ' ' | '\t' | '\n' | '\x0B' | '\x0C' | '\r')),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backslash_escapes_break_lines_and_align() {
        let display = Display::with_newlines(r"a\nb\rc\td\\e\x");
        assert_eq!(display.lines(), ["a", "b", "c\td\\e\\x"]);
        assert_eq!(
            display.natural_alignment(),
            Some(HorizontalAlignment::Right)
        );
    }

    #[test]
    fn links_keep_their_escapes() {
        assert_eq!(
            Display::with_newlines(r"[[a\nb]]\nc").lines(),
            [r"[[a\nb]]", "c"]
        );
    }

    #[test]
    fn whitespace_only_labels_are_white() {
        assert!(Display::with_newlines("  ").is_white());
        assert!(!Display::with_newlines("x").is_white());
        assert!(!Display::with_newlines(r" \n ").is_white());
    }
}
