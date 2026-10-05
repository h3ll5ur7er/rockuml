//! The lines one command consumes: a single line, or a whole block for multi-line commands.

use crate::text::StringLocated;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlocLines {
    lines: Vec<StringLocated>,
}

impl BlocLines {
    pub fn single(line: StringLocated) -> Self {
        Self { lines: vec![line] }
    }

    #[cfg(test)]
    pub fn from_texts(texts: &[&str]) -> Self {
        let location = crate::text::LineLocation::new("test", None);
        let lines = texts
            .iter()
            .map(|text| StringLocated::new(*text, location.clone()));
        Self {
            lines: lines.collect(),
        }
    }

    #[must_use]
    pub fn add(mut self, line: StringLocated) -> Self {
        self.lines.push(line);
        self
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn first(&self) -> Option<&StringLocated> {
        self.lines.first()
    }

    pub fn last(&self) -> Option<&StringLocated> {
        self.lines.last()
    }

    pub fn get(&self, index: usize) -> Option<&StringLocated> {
        self.lines.get(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &StringLocated> {
        self.lines.iter()
    }

    /// Folds a `{` written alone on the second line back onto the first, so `foo` + `{` reads as `foo {`.
    #[must_use]
    pub fn eventually_move_bracket(self) -> Self {
        let [first, second, ..] = self.lines.as_slice() else {
            return self;
        };
        if first.trimmed().text().ends_with('{') || second.trimmed().text() != "{" {
            return self;
        }
        let mut lines = self.lines;
        let joined = lines[0].append(" {");
        lines.splice(0..2, [joined]);
        Self { lines }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn texts(lines: &BlocLines) -> Vec<&str> {
        lines.iter().map(StringLocated::text).collect()
    }

    #[test]
    fn a_lone_opening_bracket_joins_the_line_before() {
        let moved =
            BlocLines::from_texts(&["package p", "  {  ", "a", "}"]).eventually_move_bracket();
        assert_eq!(texts(&moved), ["package p {", "a", "}"]);
    }

    #[test]
    fn a_line_already_ending_with_a_bracket_is_kept() {
        let lines = BlocLines::from_texts(&["package p {", "{"]);
        assert_eq!(lines.clone().eventually_move_bracket(), lines);
    }
}
