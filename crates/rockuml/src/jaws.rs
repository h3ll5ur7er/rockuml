//! Private-use characters PlantUML uses as in-band markers inside line text.

use crate::text::StringLocated;

pub const BLOCK_E1_NEWLINE: char = '\u{E100}';
pub const BLOCK_E1_NEWLINE_LEFT_ALIGN: char = '\u{E101}';
pub const BLOCK_E1_NEWLINE_RIGHT_ALIGN: char = '\u{E102}';
pub const BLOCK_E1_BREAKLINE: char = '\u{E103}';
pub const BLOCK_E1_REAL_BACKSLASH: char = '\u{E110}';
pub const BLOCK_E1_REAL_TABULATION: char = '\u{E111}';

/// `%breakline()` ends the current line: split there, except inside `{{ ... }}` embedded diagrams.
pub fn expand_breaklines(lines: Vec<StringLocated>) -> Vec<StringLocated> {
    let mut result = Vec::with_capacity(lines.len());
    for line in lines {
        if !line.text().contains(BLOCK_E1_BREAKLINE) {
            result.push(line);
            continue;
        }
        let chars: Vec<char> = line.text().chars().collect();
        let mut pending = String::new();
        let mut level = 0;
        for (index, &c) in chars.iter().enumerate() {
            let next = chars.get(index + 1).copied();
            if c == '{' && next == Some('{') {
                level += 1;
            } else if c == '}' && next == Some('}') {
                level -= 1;
            }
            if level <= 0 && c == BLOCK_E1_BREAKLINE {
                result.push(line.with_text(std::mem::take(&mut pending)));
            } else {
                pending.push(c);
            }
        }
        result.push(line.with_text(pending));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::LineLocation;

    fn texts(lines: &[StringLocated]) -> Vec<&str> {
        lines.iter().map(StringLocated::text).collect()
    }

    #[test]
    fn splits_at_breaklines_outside_embedded_diagrams() {
        let location = LineLocation::new("t", None);
        let lines = vec![
            StringLocated::new("a\u{E103}b\u{E103}", location.clone()),
            StringLocated::new("{{x\u{E103}y}}", location),
        ];
        assert_eq!(texts(&expand_breaklines(lines)), ["a", "b", "", "{{x\u{E103}y}}"]);
    }
}
