//! The chain of line readers that turns raw source text into the lines handed to block extraction.

use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::LazyLock;

use regex::Regex;

use super::start_utils;
use crate::java;
use crate::text::{LineLocation, StringLocated, ends_with_backslash};

pub(super) trait ReadLine {
    fn read_line(&mut self) -> Option<StringLocated>;
}

/// Splits text into lines the way `BufferedReader.readLine` does (`\n`, `\r` or `\r\n`).
pub(super) struct ReadLineReader {
    lines: VecDeque<String>,
    location: LineLocation,
}

impl ReadLineReader {
    pub(super) fn new(text: &str, description: &str, parent: Option<LineLocation>) -> Self {
        Self {
            lines: split_lines(text).map(str::to_owned).collect(),
            location: LineLocation::new(description, parent),
        }
    }
}

fn split_lines(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = text;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let end = rest.find(['\n', '\r']).unwrap_or(rest.len());
        let line = &rest[..end];
        rest = &rest[end..];
        rest = rest
            .strip_prefix("\r\n")
            .or_else(|| rest.strip_prefix(['\n', '\r']))
            .unwrap_or(rest);
        Some(line)
    })
}

impl ReadLine for ReadLineReader {
    fn read_line(&mut self) -> Option<StringLocated> {
        self.location = self.location.one_line_read();
        let line = self.lines.pop_front()?;
        let line = line
            .strip_prefix('\u{FEFF}')
            .unwrap_or(&line)
            .replace('\u{2013}', "-");
        Some(StringLocated::new(line, self.location.clone()))
    }
}

/// Lines given programmatically (configuration, definitions), all reported at one location.
pub(super) struct ReadLineList {
    lines: VecDeque<String>,
    location: LineLocation,
}

impl ReadLineList {
    pub(super) fn new(lines: impl IntoIterator<Item = String>, location: LineLocation) -> Self {
        Self {
            lines: lines.into_iter().collect(),
            location,
        }
    }
}

impl ReadLine for ReadLineList {
    fn read_line(&mut self) -> Option<StringLocated> {
        let line = self.lines.pop_front()?;
        Some(StringLocated::new(line, self.location.clone()))
    }
}

/// Removes the prefix that `@startuml` carries on its line (e.g. `' ` when the diagram sits inside a
/// comment block) from every following line.
pub(super) struct UncommentReadLine {
    raw: Box<dyn ReadLine>,
    header_to_remove: Option<String>,
    paused: Rc<Cell<bool>>,
}

impl UncommentReadLine {
    pub(super) fn new(raw: Box<dyn ReadLine>) -> Self {
        Self {
            raw,
            header_to_remove: None,
            paused: Rc::default(),
        }
    }

    /// Block extraction pauses and resumes the reader while it already sits inside the filter chain.
    pub(super) fn pause_switch(&self) -> Rc<Cell<bool>> {
        Rc::clone(&self.paused)
    }
}

impl ReadLine for UncommentReadLine {
    fn read_line(&mut self) -> Option<StringLocated> {
        static UNPAUSE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?i)((?:[^a-zA-Z0-9_]|<[^<>]*>)*)[@\\]unpause").unwrap());

        let result = self.raw.read_line()?;
        if let Some(header) = start_utils::before_start_uml(result.text()) {
            self.header_to_remove = Some(header.to_owned());
        }
        if self.paused.get()
            && let Some(captures) = UNPAUSE.captures(result.text())
        {
            self.header_to_remove = Some(captures[1].to_owned());
        }
        match &self.header_to_remove {
            Some(header) if header.starts_with(result.text()) => Some(result.with_text("")),
            Some(header) if result.text().starts_with(header.as_str()) => {
                Some(result.with_text(&result.text()[header.len()..]))
            }
            _ => Some(result),
        }
    }
}

/// Injects the `-config` lines right after every `@start` line.
pub(super) struct ReadFilterAddConfig {
    raw: Box<dyn ReadLine>,
    config: Vec<String>,
    inserted: VecDeque<StringLocated>,
}

impl ReadFilterAddConfig {
    pub(super) fn new(raw: Box<dyn ReadLine>, config: Vec<String>) -> Self {
        Self {
            raw,
            config,
            inserted: VecDeque::new(),
        }
    }
}

impl ReadLine for ReadFilterAddConfig {
    fn read_line(&mut self) -> Option<StringLocated> {
        if let Some(line) = self.inserted.pop_front() {
            return Some(line);
        }
        let result = self.raw.read_line()?;
        if start_utils::is_start_directive(result.text()) && !self.config.is_empty() {
            let mut config = ReadLineList::new(self.config.clone(), result.location().clone());
            while let Some(line) = read_line_skipping_quote_comments(&mut config) {
                self.inserted.push_back(line);
            }
        }
        Some(result)
    }
}

/// Joins lines ending with a backslash to the following line, skipping comment lines in between.
pub(super) struct ReadFilterMergeLines {
    source: Box<dyn ReadLine>,
    manage_ending_backslash: bool,
}

impl ReadFilterMergeLines {
    pub(super) fn new(source: Box<dyn ReadLine>) -> Self {
        Self {
            source,
            manage_ending_backslash: true,
        }
    }
}

impl ReadLine for ReadFilterMergeLines {
    fn read_line(&mut self) -> Option<StringLocated> {
        let mut result = self.source.read_line()?;
        if start_utils::is_start_directive(result.text())
            && start_utils::is_ditaa_start(java::trim(result.text()))
        {
            self.manage_ending_backslash = false;
        }
        if start_utils::is_end_directive(result.text()) {
            self.manage_ending_backslash = true;
        }
        while self.manage_ending_backslash && ends_with_backslash(result.text()) {
            let Some(next) = read_line_skipping_quote_comments(self.source.as_mut()) else {
                break;
            };
            result = result.merge_end_backslash(&next);
        }
        Some(result)
    }
}

/// Reads the next line that is not a `'` comment or part of a `/' ... '/` block comment.
pub(super) fn read_line_skipping_quote_comments(
    source: &mut dyn ReadLine,
) -> Option<StringLocated> {
    let mut inside_long_comment = false;
    loop {
        let result = source.read_line()?;
        let trimmed = result.text().replace('\t', " ");
        let trimmed = java::trim(&trimmed);
        if inside_long_comment {
            if trimmed.ends_with("'/") {
                inside_long_comment = false;
            }
            continue;
        }
        if trimmed.starts_with('\'') || (trimmed.starts_with("/'") && trimmed.ends_with("'/")) {
            continue;
        }
        if trimmed.starts_with("/'") && !trimmed.contains("'/") {
            inside_long_comment = true;
            continue;
        }
        return Some(result.remove_inner_comment());
    }
}

/// A YAML front-matter block (`---` lines right after `@start`) carries metadata, not diagram text.
pub(super) fn remove_yaml_header(lines: Vec<StringLocated>) -> Vec<StringLocated> {
    let is_separator = |line: &StringLocated| line.text() == "---";
    if lines.len() > 1
        && is_separator(&lines[1])
        && let Some(closing) = lines
            .iter()
            .skip(2)
            .position(is_separator)
            .map(|offset| offset + 2)
    {
        let mut result = vec![lines[0].clone()];
        result.extend(lines.into_iter().skip(closing + 1));
        return result;
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_all(mut reader: impl ReadLine) -> Vec<String> {
        std::iter::from_fn(|| reader.read_line())
            .map(|line| line.text().to_owned())
            .collect()
    }

    fn reader(text: &str) -> Box<dyn ReadLine> {
        Box::new(ReadLineReader::new(text, "test", None))
    }

    #[test]
    fn splits_on_every_java_line_terminator() {
        assert_eq!(
            read_all(ReadLineReader::new("a\r\nb\rc\n\nd", "t", None)),
            ["a", "b", "c", "", "d"]
        );
    }

    #[test]
    fn strips_byte_order_marks_and_normalises_en_dashes() {
        assert_eq!(
            read_all(ReadLineReader::new("\u{FEFF}A \u{2013}> B", "t", None)),
            ["A -> B"]
        );
    }

    #[test]
    fn locations_number_lines_from_zero() {
        let mut reader = ReadLineReader::new("a\nb", "t", None);
        reader.read_line();
        assert_eq!(reader.read_line().unwrap().location().position(), 1);
    }

    #[test]
    fn comment_prefix_of_the_start_line_is_removed_from_the_block() {
        let lines = read_all(UncommentReadLine::new(reader(
            "' @startuml\n' A -> B\n'\n' @enduml",
        )));
        assert_eq!(lines, [" @startuml", " A -> B", "", " @enduml"]);
    }

    #[test]
    fn continuation_lines_skip_comments() {
        let lines = read_all(ReadFilterMergeLines::new(reader("A -> \\\n' note\nB\nC")));
        assert_eq!(lines, ["A -> B", "C"]);
    }

    #[test]
    fn ditaa_blocks_keep_their_backslashes() {
        let lines = read_all(ReadFilterMergeLines::new(reader(
            "@startditaa\n+-\\\n|\n@enddittaa",
        )));
        assert_eq!(lines, ["@startditaa", "+-\\", "|", "@enddittaa"]);
    }

    #[test]
    fn config_lines_follow_each_start_line() {
        let filter = ReadFilterAddConfig::new(
            reader("@startuml\nA\n@enduml"),
            vec!["' c".into(), "skinparam x y".into()],
        );
        assert_eq!(
            read_all(filter),
            ["@startuml", "skinparam x y", "A", "@enduml"]
        );
    }

    #[test]
    fn yaml_header_after_start_line_is_dropped() {
        let location = LineLocation::new("t", None);
        let lines: Vec<_> = ["@startuml", "---", "title: x", "---", "A"]
            .map(|text| StringLocated::new(text, location.clone()))
            .into();
        let kept: Vec<_> = remove_yaml_header(lines)
            .iter()
            .map(|line| line.text().to_owned())
            .collect();
        assert_eq!(kept, ["@startuml", "A"]);
    }
}
