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
        /// A line break, and the alignment it asks for.
        enum Break {
            Plain,
            Aligned(HorizontalAlignment),
        }
        let mut lines = vec![String::new()];
        let mut natural_alignment = None;
        let mut raw = false;
        let mut chars = text.char_indices().peekable();
        while let Some((at, c)) = chars.next() {
            let rest = &text[at..];
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
            let current = lines.last_mut().expect("there is always a current line");
            let line_break = match c {
                '\\' if !raw && chars.peek().is_some() => match chars.next().expect("peeked").1 {
                    'n' => Some(Break::Plain),
                    'r' => Some(Break::Aligned(HorizontalAlignment::Right)),
                    'l' => Some(Break::Aligned(HorizontalAlignment::Left)),
                    escaped => {
                        match escaped {
                            't' => current.push('\t'),
                            '\\' => current.push('\\'),
                            other => {
                                current.push('\\');
                                current.push(other);
                            }
                        }
                        None
                    }
                },
                BLOCK_E1_NEWLINE_LEFT_ALIGN => Some(Break::Aligned(HorizontalAlignment::Left)),
                BLOCK_E1_NEWLINE_RIGHT_ALIGN => Some(Break::Aligned(HorizontalAlignment::Right)),
                BLOCK_E1_NEWLINE if !raw => Some(Break::Plain),
                BLOCK_E1_BREAKLINE => Some(Break::Plain),
                BLOCK_E1_REAL_BACKSLASH => {
                    current.push('\\');
                    None
                }
                BLOCK_E1_INVISIBLE_QUOTE => None,
                _ => {
                    current.push(c);
                    None
                }
            };
            match line_break {
                Some(Break::Aligned(alignment)) => {
                    natural_alignment = Some(alignment);
                    lines.push(String::new());
                }
                Some(Break::Plain) => lines.push(String::new()),
                None => {}
            }
        }
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
            [only] => only.chars().all(crate::java::is_regex_whitespace),
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
