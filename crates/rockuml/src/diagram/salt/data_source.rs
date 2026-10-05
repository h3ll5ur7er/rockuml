use std::sync::LazyLock;

use regex::Regex;

use crate::java;
use crate::pattern::plantuml_regex;

/// How an item ends: the next item goes in the next column, or starts the next row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Terminator {
    NewColumn,
    NewLine,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Terminated<T> {
    pub item: T,
    pub terminator: Terminator,
}

/// The salt source cut into cells: `|` separates columns, line ends rows, `{` and `}` open and close
/// groups (PlantUML's `DataSourceImpl`).
pub(super) struct DataSource {
    items: Vec<Terminated<String>>,
    position: usize,
}

const ESCAPED_PIPE: char = '\u{E000}';
const ESCAPED_OPEN: char = '\u{E001}';
const ESCAPED_CLOSE: char = '\u{E002}';

impl DataSource {
    pub(super) fn new(lines: &[String]) -> Self {
        static BLOCK_START: LazyLock<Regex> =
            LazyLock::new(|| plantuml_regex(r"\{(?:[-+^#!*/]|S-|SI|S)?"));
        let mut items = Vec::new();
        let mut add = |text: &str, terminator| {
            let text = java::trim(text)
                .replace(ESCAPED_PIPE, "|")
                .replace(ESCAPED_OPEN, "{")
                .replace(ESCAPED_CLOSE, "}");
            if !text.is_empty() {
                items.push(Terminated {
                    item: text,
                    terminator,
                });
            }
        };
        for line in lines {
            let line = line
                .replace("\\|", &ESCAPED_PIPE.to_string())
                .replace("\\{", &ESCAPED_OPEN.to_string())
                .replace("\\}", &ESCAPED_CLOSE.to_string());
            let tokens = tokens_with_delimiters(&line, &['|', '}']);
            for (index, token) in tokens.iter().enumerate() {
                let token = java::trim(token);
                if token == "|" {
                    continue;
                }
                let terminator = if index + 1 < tokens.len() {
                    Terminator::NewColumn
                } else {
                    Terminator::NewLine
                };
                let mut last_start = 0;
                let mut end = 0;
                for found in BLOCK_START.find_iter(token) {
                    if found.start() > last_start {
                        add(&token[last_start..found.start()], Terminator::NewColumn);
                    }
                    end = found.end();
                    let at_end = if end == token.len() {
                        terminator
                    } else {
                        Terminator::NewColumn
                    };
                    add(found.as_str(), at_end);
                    last_start = end;
                }
                if end < token.len() {
                    add(&token[end..], terminator);
                }
            }
        }
        Self { items, position: 0 }
    }

    /// The item `ahead` places from the current one. Like PlantUML, fails past the end.
    pub(super) fn peek(&self, ahead: usize) -> Option<&Terminated<String>> {
        self.items.get(self.position + ahead)
    }

    pub(super) fn next(&mut self) -> Option<Terminated<String>> {
        let item = self.items.get(self.position).cloned();
        self.position += 1;
        item
    }
}

/// `StringTokenizer` with `returnDelims`: the pieces between delimiters and each delimiter on its own.
fn tokens_with_delimiters<'a>(text: &'a str, delimiters: &[char]) -> Vec<&'a str> {
    let mut tokens = Vec::new();
    let mut start = 0;
    for (index, c) in text.char_indices() {
        if delimiters.contains(&c) {
            if index > start {
                tokens.push(&text[start..index]);
            }
            tokens.push(&text[index..index + c.len_utf8()]);
            start = index + c.len_utf8();
        }
    }
    if start < text.len() {
        tokens.push(&text[start..]);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(lines: &[&str]) -> Vec<(String, Terminator)> {
        let lines: Vec<String> = lines.iter().map(|line| (*line).to_owned()).collect();
        let mut source = DataSource::new(&lines);
        std::iter::from_fn(|| source.next())
            .map(|item| (item.item, item.terminator))
            .collect()
    }

    #[test]
    fn cells_are_split_at_pipes_and_braces() {
        use Terminator::{NewColumn, NewLine};
        assert_eq!(
            items(&["{+", "  Login | \"MyName\"", "  [OK] \\| x }"]),
            [
                ("{+".to_owned(), NewLine),
                ("Login".to_owned(), NewColumn),
                ("\"MyName\"".to_owned(), NewLine),
                ("[OK] | x".to_owned(), NewColumn),
                ("}".to_owned(), NewLine),
            ]
        );
    }

    #[test]
    fn a_block_start_inside_a_cell_is_cut_out() {
        use Terminator::{NewColumn, NewLine};
        assert_eq!(
            items(&["a {# b"]),
            [
                ("a".to_owned(), NewColumn),
                ("{#".to_owned(), NewColumn),
                ("b".to_owned(), NewLine),
            ]
        );
    }
}
