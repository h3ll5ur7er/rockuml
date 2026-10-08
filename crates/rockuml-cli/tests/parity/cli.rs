//! The command line against the golden model: every scenario in `tests/cli` must exit, print and write files
//! as PlantUML did when `tools/oracle/cli-goldens.sh` recorded it.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::check::{embedded_pngs_as_pixels, first_difference, normalise, png_size};
use crate::repository_root;

const WORKDIR: &str = "<WORKDIR>";

#[test]
fn rockuml_runs_every_cli_scenario_like_the_golden_model() {
    let mut scenarios: Vec<PathBuf> = fs::read_dir(repository_root().join("tests/cli"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.join("args").is_file())
        .collect();
    scenarios.sort();
    assert_ne!(scenarios.len(), 0, "no scenarios in tests/cli");

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
    let output = run(scenario, work.path(), &workdir);

    let expected = scenario.join("expected");
    let expected_status: i32 = fs::read_to_string(expected.join("status"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    if output.status.code() != Some(expected_status) {
        return Err(format!(
            "exit status {:?} instead of {expected_status}; stderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    for (stream, produced) in [("stdout", &output.stdout), ("stderr", &output.stderr)] {
        let golden = fs::read(expected.join(stream)).unwrap();
        compare(
            stream,
            &comparable(&golden, WORKDIR),
            &comparable(produced, &workdir),
        )?;
    }
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
            &comparable(content, WORKDIR),
            &comparable(&produced_files[name], &workdir),
        )?;
    }
    Ok(())
}

/// rockuml run in `work` with the scenario's arguments, environment and standard input.
fn run(scenario: &Path, work: &Path, workdir: &str) -> std::process::Output {
    let arguments: Vec<String> = fs::read_to_string(scenario.join("args"))
        .unwrap()
        .lines()
        .map(|argument| argument.replace(WORKDIR, workdir))
        .collect();
    let mut command = Command::new(env!("CARGO_BIN_EXE_rockuml"));
    command
        .args(&arguments)
        .current_dir(work)
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
    // rockuml may stop reading standard input early, so writing it may fail.
    let writer = std::thread::spawn(move || child_stdin.write_all(&stdin));
    let output = child.wait_with_output().unwrap();
    let _ = writer.join().unwrap();
    output
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

/// The files below `root` by their path relative to it, written with `/`, without those identical to the
/// file of the same path in `unchanged_from`.
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
            let unchanged = unchanged_from.is_some_and(|input| {
                fs::read(input.join(relative)).is_ok_and(|old| old == content)
            });
            if !unchanged {
                let name = relative
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                result.insert(name, content);
            }
        }
    }
    result
}

/// What may differ without saying anything about the command line, besides what `normalise` masks: path
/// separators, where the run happened, and how PNGs are encoded (compared by size, or by pixels when
/// embedded). Text in an encoding other than UTF-8 compares byte for byte.
fn comparable(content: &[u8], workdir: &str) -> String {
    if content.starts_with(b"\x89PNG") {
        return png_size(content);
    }
    let text = match content {
        [0xFE, 0xFF, rest @ ..] => utf16(rest, u16::from_be_bytes),
        [0xFF, 0xFE, rest @ ..] => utf16(rest, u16::from_le_bytes),
        _ => {
            let Ok(text) = std::str::from_utf8(content) else {
                let without_carriage_returns: Vec<u8> = content
                    .iter()
                    .copied()
                    .filter(|&byte| byte != b'\r')
                    .collect();
                return format!("{without_carriage_returns:?}");
            };
            text.to_owned()
        }
    };
    let text = normalise(&text)
        .replace(workdir, WORKDIR)
        .replace('\\', "/");
    embedded_pngs_as_pixels(&text)
}

fn utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&pair| unit(pair))
        .collect();
    String::from_utf16_lossy(&units)
}

fn compare(what: &str, expected: &str, produced: &str) -> Result<(), String> {
    match first_difference(expected, produced) {
        Some(difference) => Err(format!("{what}: {difference}")),
        None => Ok(()),
    }
}
