use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::LazyLock;

use regex::Regex;

use crate::corpus::{Case, GoldenKind, file_name};

pub enum Outcome {
    Pass,
    Fail(String),
}

/// Returns `None` when the golden model produced nothing of this kind for the case.
pub fn check(rockuml: &Path, case: &Case, kind: GoldenKind) -> Option<Outcome> {
    let goldens = case.goldens(kind);
    if goldens.is_empty() {
        return None;
    }

    let output_directory = tempfile::tempdir().unwrap();
    let run = Command::new(rockuml)
        .args(kind.cli_arguments())
        .arg("-o")
        .arg(output_directory.path())
        .arg(&case.source)
        .output()
        .unwrap();

    let produced: BTreeMap<String, String> = fs::read_dir(output_directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .map(|path| (file_name(&path), read_normalised(&path)))
        .collect();

    if produced.is_empty() {
        return Some(Outcome::Fail(format!(
            "produced no output ({}): {}",
            run.status,
            String::from_utf8_lossy(&run.stderr).trim()
        )));
    }

    Some(compare(
        &goldens
            .into_iter()
            .map(|(name, path)| (name, read_normalised(&path)))
            .collect(),
        &produced,
    ))
}

fn compare(expected: &BTreeMap<String, String>, produced: &BTreeMap<String, String>) -> Outcome {
    let expected_names: Vec<_> = expected.keys().collect();
    let produced_names: Vec<_> = produced.keys().collect();
    if expected_names != produced_names {
        return Outcome::Fail(format!(
            "expected files {expected_names:?}, produced {produced_names:?}"
        ));
    }

    for (name, expected_content) in expected {
        if let Some(difference) = first_difference(expected_content, &produced[name]) {
            return Outcome::Fail(format!("{name}: {difference}"));
        }
    }
    Outcome::Pass
}

fn first_difference(expected: &str, produced: &str) -> Option<String> {
    let mut expected_lines = expected.lines();
    let mut produced_lines = produced.lines();
    for line_number in 1.. {
        match (expected_lines.next(), produced_lines.next()) {
            (None, None) => return None,
            (expected_line, produced_line) if expected_line != produced_line => {
                return Some(format!(
                    "line {line_number}: expected {:?}, produced {:?}",
                    expected_line.unwrap_or("<end of file>"),
                    produced_line.unwrap_or("<end of file>")
                ));
            }
            _ => {}
        }
    }
    unreachable!()
}

fn read_normalised(path: &Path) -> String {
    normalise(&String::from_utf8_lossy(&fs::read(path).unwrap()))
}

/// Line endings depend on how git checked out the goldens, and PlantUML's debug output stamps the
/// current time next to shapes it cannot describe; neither says anything about rendering.
fn normalise(text: &str) -> String {
    static RENDER_TIMESTAMP: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(Mon|Tue|Wed|Thu|Fri|Sat|Sun) [A-Z][a-z]{2} \d{2} \d{2}:\d{2}:\d{2} \S+ \d{4}")
            .unwrap()
    });
    RENDER_TIMESTAMP
        .replace_all(&text.replace("\r\n", "\n"), "<timestamp>")
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_the_first_differing_line() {
        assert_eq!(
            first_difference("a\nb\nc", "a\nx\nc").as_deref(),
            Some(r#"line 2: expected "b", produced "x""#)
        );
    }

    #[test]
    fn reports_missing_trailing_lines() {
        assert_eq!(
            first_difference("a\nb", "a").as_deref(),
            Some(r#"line 2: expected "b", produced "<end of file>""#)
        );
    }

    #[test]
    fn identical_text_has_no_difference() {
        assert_eq!(first_difference("a\nb\n", "a\nb\n"), None);
    }

    #[test]
    fn masks_render_timestamps_and_line_endings() {
        assert_eq!(
            normalise(
                "  backcolor: HColorGradient Mon Oct 05 14:49:18 CEST 2026\r\nUGraphicDebug UImage Sun Jan 01 09:00:00 GMT+02:00 2027\r\n"
            ),
            "  backcolor: HColorGradient <timestamp>\nUGraphicDebug UImage <timestamp>\n"
        );
    }
}
