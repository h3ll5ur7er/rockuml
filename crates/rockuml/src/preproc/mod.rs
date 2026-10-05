//! Turns source text into preprocessed diagram blocks.

mod block;
mod reader;
mod start_utils;

use std::sync::LazyLock;

use regex::Regex;

use reader::{ReadFilterMergeLines, ReadLine, ReadLineReader, UncommentReadLine};

use std::path::PathBuf;

use crate::host::Host;
use crate::java;
use crate::pattern::plantuml_regex;
use crate::text::{LineLocation, StringLocated};
use crate::tim::{self, Definitions, Folder};

pub use crate::tim::{PreprocessorEnvironment, java_date_string};

/// A diagram source file, or any text standing in for one.
pub struct Source<'a> {
    pub text: &'a str,
    /// Names the source in error messages; for files, the file name.
    pub description: &'a str,
    /// Relative `!include`s resolve against this directory.
    pub directory: PathBuf,
    pub environment: PreprocessorEnvironment,
}

/// One diagram's source after preprocessing, from its `@start` line to its `@end` line.
pub struct PreprocessedBlock {
    lines: Vec<StringLocated>,
    failed: bool,
}

impl PreprocessedBlock {
    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.lines.iter().map(StringLocated::text)
    }

    /// The lines joined as PlantUML does when it encodes a diagram's source: each followed by `\n`.
    pub fn source_text(&self) -> String {
        self.lines().flat_map(|line| [line, "\n"]).collect()
    }

    /// Whether preprocessing stopped at an error, reported on the last line.
    pub fn failed(&self) -> bool {
        self.failed
    }

    /// The output name given after `@startuml`, e.g. `@startuml diagram.png` names the output `diagram`.
    pub fn output_name(&self) -> Option<String> {
        static FILENAME: LazyLock<Regex> = LazyLock::new(|| {
            plantuml_regex("^[@\\\\]start[^%s{}%g]+[%s{][%s%g]*([^%g]*?)[%s}%g]*$")
        });
        static EXTENSION: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\.[a-zA-Z0-9_]{3}$").unwrap());

        let first_line = java::trim(self.lines.first()?.text());
        let captures = FILENAME.captures(first_line)?;
        let mut name = &captures[1];
        if let Some(comma) = name.find(',') {
            name = &name[..comma];
        }
        if name.contains(['<', '>', '|']) {
            return None;
        }
        let name = name.strip_prefix("file://").unwrap_or(name);
        Some(EXTENSION.replace(name, "").into_owned())
    }
}

/// The lines of an included resource: the inside of one of its diagram blocks if it has any (read with
/// continuation lines merged and comment prefixes removed), otherwise every line as written.
///
/// `selector` picks the block: a number counts blocks from zero, anything else matches `id=...` on the
/// start line. Without one, the first block is taken; if no block matches, nothing is.
pub(crate) fn read_lines_of_diagram(
    text: &str,
    extracted_description: &str,
    plain_description: &str,
    parent: Option<LineLocation>,
    selector: Option<&str>,
) -> Vec<StringLocated> {
    let detected = read_all(UncommentReadLine::new(Box::new(ReadFilterMergeLines::new(
        Box::new(ReadLineReader::new(text, extracted_description, None)),
    ))));
    if !detected
        .iter()
        .any(|line| start_utils::is_start_directive(line.text()))
    {
        return read_plain_lines(text, plain_description, parent);
    }
    let mut starts = detected
        .iter()
        .enumerate()
        .filter(|(_, line)| {
            start_utils::is_start_directive(line.text()) && selects_by_id(selector, line)
        })
        .map(|(index, _)| index);
    let start = match selector
        .filter(|selector| !selector.is_empty() && selector.bytes().all(|b| b.is_ascii_digit()))
    {
        Some(number) => number
            .parse()
            .ok()
            .and_then(|block: usize| starts.nth(block)),
        None => starts.next(),
    };
    let Some(start) = start else {
        return Vec::new();
    };
    detected
        .into_iter()
        .skip(start + 1)
        .take_while(|line| !start_utils::is_end_directive(line.text()))
        .collect()
}

/// PlantUML tests the id against the line's debug form, `(SL) <text>`.
fn selects_by_id(selector: Option<&str>, line: &StringLocated) -> bool {
    let Some(id) = selector.filter(|selector| !selector.bytes().all(|b| b.is_ascii_digit())) else {
        return true;
    };
    let shown = if line.text().is_empty() {
        "<<<EMPTY STRING>>>".to_owned()
    } else {
        format!("(SL) {}", line.text())
    };
    crate::pattern::try_java_regex(&format!(r"^.*id={id}\W.*$"), false)
        .is_some_and(|regex| regex.is_match(&shown))
}

/// Every line as written: no comment prefix removal, no continuation merging.
pub(crate) fn read_plain_lines(
    text: &str,
    description: &str,
    parent: Option<LineLocation>,
) -> Vec<StringLocated> {
    read_all(ReadLineReader::new(text, description, parent))
}

/// An `!includesub` source: comment prefixes removed, then continuation lines merged.
pub(crate) fn read_uncommented_merged_lines(
    text: &str,
    description: &str,
    parent: Option<LineLocation>,
) -> Vec<StringLocated> {
    read_all(ReadFilterMergeLines::new(Box::new(UncommentReadLine::new(
        Box::new(ReadLineReader::new(text, description, parent)),
    ))))
}

/// Lines with the comment prefix of the `@start` line removed, nothing else changed.
pub(crate) fn read_uncommented_lines(text: &str, description: &str) -> Vec<String> {
    read_all(UncommentReadLine::new(Box::new(ReadLineReader::new(
        text,
        description,
        None,
    ))))
    .into_iter()
    .map(|line| line.text().to_owned())
    .collect()
}

fn read_all(mut reader: impl ReadLine) -> Vec<StringLocated> {
    std::iter::from_fn(|| reader.read_line()).collect()
}

/// Splits the source into diagram blocks and preprocesses each in turn.
pub fn preprocess(source: &Source, host: &dyn Host) -> Vec<PreprocessedBlock> {
    let mut blocks: Vec<PreprocessedBlock> = Vec::new();
    for lines in block::extract_blocks(source.text, source.description, Vec::new()) {
        let preprocessed = tim::preprocess_block(
            &lines,
            host,
            &source.environment,
            &EarlierDefinitions(&blocks),
            Folder::Regular(source.directory.clone()),
        );
        blocks.push(PreprocessedBlock {
            lines: preprocessed.lines,
            failed: preprocessed.failed,
        });
    }
    blocks
}

/// `@startdef(id=NAME)` blocks that precede the one being preprocessed.
struct EarlierDefinitions<'a>(&'a [PreprocessedBlock]);

impl Definitions for EarlierDefinitions<'_> {
    fn definition(&self, name: &str) -> Vec<String> {
        let signature = format!("@startdef(id={name})");
        self.0
            .iter()
            .find(|block| {
                block
                    .lines
                    .first()
                    .is_some_and(|first| first.text().eq_ignore_ascii_case(&signature))
            })
            .map(|block| {
                let inner = &block.lines[1..block.lines.len().saturating_sub(1).max(1)];
                inner.iter().map(|line| line.text().to_owned()).collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preprocess_text(text: &str) -> Vec<PreprocessedBlock> {
        let source = Source {
            text,
            description: "t",
            directory: PathBuf::new(),
            environment: PreprocessorEnvironment::default(),
        };
        preprocess(&source, &crate::host::IsolatedHost)
    }

    fn output_name(source: &str) -> Option<String> {
        preprocess_text(source).remove(0).output_name()
    }

    /// Serves one document at `https://example.com/lib.puml`.
    struct WebHost;

    impl crate::host::Host for WebHost {
        fn read_file(&self, _: &std::path::Path) -> Option<Vec<u8>> {
            None
        }
        fn read_url(&self, url: &str) -> Option<Vec<u8>> {
            let document = "@startuml(id=first)\nFirst -> Block\n@enduml\n@startuml(id=second)\nSecond -> Block\n@enduml\n";
            (url == "https://example.com/lib.puml").then(|| document.as_bytes().to_vec())
        }
        fn file_exists(&self, _: &std::path::Path) -> bool {
            false
        }
        fn current_directory(&self) -> PathBuf {
            PathBuf::new()
        }
        fn home_directory(&self) -> Option<PathBuf> {
            None
        }
        fn getenv(&self, _: &str) -> Option<String> {
            None
        }
        fn current_time_millis(&self) -> i64 {
            0
        }
        fn local_time_zone(&self) -> Option<String> {
            None
        }
    }

    fn preprocess_on_web(include: &str) -> Vec<String> {
        let text = format!("@startuml\n{include}\n@enduml");
        let source = Source {
            text: &text,
            description: "t",
            directory: PathBuf::new(),
            environment: PreprocessorEnvironment::default(),
        };
        preprocess(&source, &WebHost)
            .remove(0)
            .lines()
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn url_includes_take_the_selected_diagram_block() {
        let expected = |line: &str| {
            [
                "@startuml".to_owned(),
                line.to_owned(),
                "@enduml".to_owned(),
            ]
        };
        assert_eq!(
            preprocess_on_web("!include https://example.com/lib.puml"),
            expected("First -> Block")
        );
        assert_eq!(
            preprocess_on_web("!includeurl https://example.com/lib.puml!1"),
            expected("Second -> Block")
        );
        assert_eq!(
            preprocess_on_web("!include https://example.com/lib.puml!second"),
            expected("Second -> Block")
        );
    }

    #[test]
    fn unreachable_or_forbidden_urls_cannot_be_included() {
        let failing = |include: &str| preprocess_on_web(include).last().cloned();
        assert_eq!(
            failing("!include https://example.com/missing.puml").as_deref(),
            Some("!include https://example.com/missing.puml")
        );
        assert_eq!(
            failing("!include http://localhost/lib.puml").as_deref(),
            Some("!include http://localhost/lib.puml")
        );
    }

    fn preprocessed(lines: &[&str]) -> Vec<String> {
        preprocess_text(&lines.join("\n"))
            .remove(0)
            .lines()
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn variables_and_functions_are_expanded() {
        let source = [
            "@startuml",
            "!$x = 2",
            "!function $twice($v)",
            "!return $v * 2",
            "!endfunction",
            "A -> B : $twice($x)",
            "@enduml",
        ];
        assert_eq!(
            preprocessed(&source),
            ["@startuml", "A -> B : 4", "@enduml"]
        );
    }

    #[test]
    fn errors_end_preprocessing_at_the_failing_line() {
        let source = ["@startuml", "!assert 0 : \"boom\"", "A -> B", "@enduml"].join("\n");
        let block = preprocess_text(&source).remove(0);
        assert!(block.failed());
        assert_eq!(
            block.lines().collect::<Vec<_>>(),
            ["@startuml", "!assert 0 : \"boom\""]
        );
    }

    #[test]
    fn output_name_comes_from_the_start_line() {
        assert_eq!(
            output_name("@startuml diagram.png\n@enduml").as_deref(),
            Some("diagram")
        );
        assert_eq!(
            output_name("@startuml \"out/name\"\n@enduml").as_deref(),
            Some("out/name")
        );
        assert_eq!(output_name("@startuml(id=foo)\n@enduml"), None);
        assert_eq!(output_name("@startuml\n@enduml"), None);
    }
}
