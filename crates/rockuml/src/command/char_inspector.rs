//! Reads a block's trimmed lines one character at a time, as one text without line breaks (PlantUML's
//! `CharInspectorImpl` without newlines).
//!
//! PlantUML reads UTF-16 units, so a supplementary character is two surrogate halves to it. Here it comes
//! whole, which keeps the text it is read into whole; readers test letters with `java::is_one_char` to
//! decide as PlantUML does. Where PlantUML parts the pair, it draws lone halves that no `&str` holds.

use super::BlocLines;

/// Jumping past the end, where PlantUML throws `IllegalStateException`.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PastTheEnd;

impl PastTheEnd {
    pub(crate) const EXCEPTION: &str = "java.lang.IllegalStateException";
}

pub(crate) struct CharInspector {
    lines: Vec<Vec<char>>,
    /// `None` once every line is read.
    line: Option<usize>,
    pos: usize,
}

impl CharInspector {
    pub(crate) fn new(lines: &BlocLines) -> Self {
        Self {
            lines: lines
                .iter()
                .map(|line| line.trimmed().text().chars().collect())
                .collect(),
            line: Some(0),
            pos: 0,
        }
    }

    /// The character `ahead` of the current one on its line, or `'\0'` past the line's end.
    pub(crate) fn peek(&self, ahead: usize) -> char {
        self.line
            .and_then(|line| self.lines.get(line))
            .and_then(|line| line.get(self.pos + ahead).copied())
            .unwrap_or('\0')
    }

    /// Moves to the next character, skipping empty lines.
    pub(crate) fn jump(&mut self) -> Result<(), PastTheEnd> {
        let mut line = self.line.ok_or(PastTheEnd)?;
        self.pos += 1;
        if self.pos >= self.lines.get(line).map_or(0, Vec::len) {
            line += 1;
            self.pos = 0;
        }
        while self.lines.get(line).is_some_and(Vec::is_empty) {
            line += 1;
        }
        self.line = (line < self.lines.len()).then_some(line);
        Ok(())
    }
}

impl BlocLines {
    pub(crate) fn inspector(&self) -> CharInspector {
        CharInspector::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_all(texts: &[&str]) -> (String, Result<(), PastTheEnd>) {
        let mut inspector = BlocLines::from_texts(texts).inspector();
        let mut read = String::new();
        loop {
            read.push(inspector.peek(0));
            if let Err(error) = inspector.jump() {
                return (read, Err(error));
            }
        }
    }

    #[test]
    fn lines_are_trimmed_and_joined_and_empty_ones_skipped() {
        let (read, end) = read_all(&["  ab ", "", "c"]);
        assert_eq!(read, "abc\0");
        assert_eq!(end, Err(PastTheEnd));
    }

    #[test]
    fn peeking_stops_at_the_end_of_the_line() {
        let inspector = BlocLines::from_texts(&["ab", "cd"]).inspector();
        assert_eq!(
            [inspector.peek(0), inspector.peek(1), inspector.peek(2)],
            ['a', 'b', '\0']
        );
    }
}
