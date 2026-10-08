//! Acceptance tests of the command line against the golden model: every scenario in `tests/cli` must exit,
//! print and write files as PlantUML did when `tools/oracle/cli-goldens.sh` recorded it.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex};

use regex::Regex;

const WORKDIR: &str = "<WORKDIR>";

fn scenarios_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/cli")
}

#[test]
fn rockuml_runs_every_scenario_like_the_golden_model() {
    let mut scenarios: Vec<PathBuf> = fs::read_dir(scenarios_directory())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.join("args").is_file())
        .collect();
    scenarios.sort();
    assert!(!scenarios.is_empty());

    let failures = Mutex::new(Vec::new());
    let next = AtomicUsize::new(0);
    let workers = std::thread::available_parallelism().map_or(4, usize::from);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                while let Some(scenario) = scenarios.get(next.fetch_add(1, Ordering::Relaxed)) {
                    if let Err(difference) = check(scenario) {
                        let name = scenario.file_name().unwrap().to_string_lossy().into_owned();
                        failures
                            .lock()
                            .unwrap()
                            .push(format!("{name}: {difference}"));
                    }
                }
            });
        }
    });
    let mut failures = failures.into_inner().unwrap();
    failures.sort();
    assert!(
        failures.is_empty(),
        "{} of {} scenarios differ:\n{}",
        failures.len(),
        scenarios.len(),
        failures.join("\n")
    );
}

fn check(scenario: &Path) -> Result<(), String> {
    let work = tempfile::tempdir().unwrap();
    let input = scenario.join("input");
    if input.is_dir() {
        copy_tree(&input, work.path());
    }
    let workdir = work.path().to_string_lossy().into_owned();
    let arguments: Vec<String> = fs::read_to_string(scenario.join("args"))
        .unwrap()
        .lines()
        .map(|argument| argument.replace(WORKDIR, &workdir))
        .collect();
    let mut command = Command::new(env!("CARGO_BIN_EXE_rockuml"));
    command
        .args(&arguments)
        .current_dir(work.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for line in fs::read_to_string(scenario.join("env"))
        .unwrap_or_default()
        .lines()
    {
        let (name, value) = line.split_once('=').unwrap();
        command.env(name, value);
    }
    let mut child = command.spawn().unwrap();
    let stdin = fs::read(scenario.join("stdin")).unwrap_or_default();
    let mut child_stdin = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || child_stdin.write_all(&stdin));
    let output = child.wait_with_output().unwrap();
    // rockuml may stop reading once it has failed.
    let _ = writer.join().unwrap();

    let expected = scenario.join("expected");
    let expected_status: i32 = fs::read_to_string(expected.join("status"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.code() != Some(expected_status) {
        return Err(format!(
            "exit status {:?} instead of {expected_status}; stderr: {}",
            output.status.code(),
            stderr.trim()
        ));
    }
    compare(
        "stdout",
        &normalise(&fs::read(expected.join("stdout")).unwrap(), WORKDIR),
        &normalise(&output.stdout, &workdir),
    )?;
    compare(
        "stderr",
        &normalise(&fs::read(expected.join("stderr")).unwrap(), WORKDIR),
        &normalise(&output.stderr, &workdir),
    )?;
    let expected_files = files_in(&expected.join("files"), None);
    let produced_files = files_in(work.path(), Some(&input));
    let expected_names: Vec<_> = expected_files.keys().collect();
    let produced_names: Vec<_> = produced_files.keys().collect();
    if expected_names != produced_names {
        return Err(format!(
            "expected files {expected_names:?}, produced {produced_names:?}"
        ));
    }
    for (name, content) in &expected_files {
        compare(
            name,
            &normalise(content, WORKDIR),
            &normalise(&produced_files[name], &workdir),
        )?;
    }
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) {
    for entry in fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let target = to.join(path.file_name().unwrap());
        if path.is_dir() {
            fs::create_dir_all(&target).unwrap();
            copy_tree(&path, &target);
        } else {
            fs::copy(&path, &target).unwrap();
        }
    }
}

/// The files below `root` by their path relative to it, written with `/`; those identical to the file of
/// the same path in `unchanged_from` are left out.
fn files_in(root: &Path, unchanged_from: Option<&Path>) -> BTreeMap<String, Vec<u8>> {
    let mut result = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let relative = path.strip_prefix(root).unwrap();
            let content = fs::read(&path).unwrap();
            if unchanged_from
                .is_some_and(|input| fs::read(input.join(relative)).ok() == Some(content.clone()))
            {
                continue;
            }
            let name = relative
                .components()
                .map(|part| part.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            result.insert(name, content);
        }
    }
    result
}

/// What may differ between the golden model's run and rockuml's without saying anything about the command
/// line: line endings, path separators, where the run happened, render timestamps, and how PNGs are drawn
/// (compared by size). Text in another encoding than UTF-8 compares byte for byte.
fn normalise(content: &[u8], workdir: &str) -> String {
    static RENDER_TIMESTAMP: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(Mon|Tue|Wed|Thu|Fri|Sat|Sun) [A-Z][a-z]{2} \d{2} \d{2}:\d{2}:\d{2} \S+ \d{4}")
            .unwrap()
    });
    if content.starts_with(b"\x89PNG") {
        let dimension = |at: usize| u32::from_be_bytes(content[at..at + 4].try_into().unwrap());
        return format!("png {} x {}", dimension(16), dimension(20));
    }
    let text = match content {
        [0xFE, 0xFF, rest @ ..] => utf16(rest, u16::from_be_bytes),
        [0xFF, 0xFE, rest @ ..] => utf16(rest, u16::from_le_bytes),
        _ => match std::str::from_utf8(content) {
            Ok(text) => text.to_owned(),
            Err(_) => {
                return format!(
                    "{:?}",
                    content
                        .split(|&byte| byte == b'\r')
                        .collect::<Vec<_>>()
                        .concat()
                );
            }
        },
    };
    let text = text
        .replace("\r\n", "\n")
        .replace(workdir, WORKDIR)
        .replace('\\', "/");
    RENDER_TIMESTAMP
        .replace_all(&text, "<timestamp>")
        .into_owned()
}

fn utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| unit([pair[0], pair[1]]))
        .collect();
    String::from_utf16_lossy(&units)
}

fn compare(what: &str, expected: &str, produced: &str) -> Result<(), String> {
    let mut expected_lines = expected.lines();
    let mut produced_lines = produced.lines();
    for line_number in 1.. {
        match (expected_lines.next(), produced_lines.next()) {
            (None, None) => return Ok(()),
            (expected_line, produced_line) if expected_line != produced_line => {
                return Err(format!(
                    "{what} line {line_number}: expected {:?}, produced {:?}",
                    expected_line.unwrap_or("<end>"),
                    produced_line.unwrap_or("<end>")
                ));
            }
            _ => {}
        }
    }
    unreachable!()
}
