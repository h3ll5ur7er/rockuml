//! Graphviz's `dot`, which `@startdot` diagrams run: where PlantUML looks for it and how it runs it
//! (PlantUML's `GraphvizRuntimeEnvironment`, `AbstractGraphviz`, `GraphvizLinux`, `GraphvizWindowsOld`,
//! `ExeState` and `ProcessRunner`).

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use rockuml::host::Graphviz;

use crate::cli_options::eventually_remove_starting_and_ending_double_quote;

/// The environment variable that names `dot` when the command line does not.
pub(crate) const GRAPHVIZ_DOT: &str = "GRAPHVIZ_DOT";

/// The `dot` that `named`, what the command line names, or else the environment's `GRAPHVIZ_DOT` name
/// (`getenvGraphvizDot`).
pub(crate) fn getenv_graphviz_dot(named: Option<&str>) -> Option<String> {
    let getenv = std::env::var(GRAPHVIZ_DOT).ok();
    named
        .filter(|path| !path.is_empty())
        .or_else(|| getenv.as_deref().filter(|path| !path.is_empty()))
        .map(|path| eventually_remove_starting_and_ending_double_quote(path).to_owned())
}

/// One run of `dot` on a Graphviz source (`AbstractGraphviz`).
pub(crate) struct AbstractGraphviz {
    dot_exe: Option<PathBuf>,
    dot_string: String,
    format: String,
}

impl AbstractGraphviz {
    /// Without `getenv_graphviz_dot`, `dot` is looked for on the `PATH`, then where installers put it.
    pub(crate) fn new(getenv_graphviz_dot: Option<String>, dot_string: &str, format: &str) -> Self {
        Self {
            dot_exe: search_dot_exe(getenv_graphviz_dot),
            dot_string: dot_string.to_owned(),
            format: format.to_owned(),
        }
    }
}

impl Graphviz for AbstractGraphviz {
    fn exe_state_is_ok(&self) -> bool {
        self.dot_exe.as_deref().is_some_and(|dot_exe| {
            dot_exe.exists() && !dot_exe.is_dir() && dot_exe.is_file() && can_read(dot_exe)
        })
    }

    fn create_file3(&self) -> Option<Vec<u8>> {
        let dot_exe = std::path::absolute(self.dot_exe.as_deref()?).ok()?;
        // Like Java's `redirectErrorStream`, errors and warnings go into the output.
        let (mut output, output_writer) = std::io::pipe().ok()?;
        let mut child = Command::new(dot_exe)
            .arg(format!("-T{}", self.format))
            .stdin(Stdio::piped())
            .stdout(output_writer.try_clone().ok()?)
            .stderr(output_writer)
            .spawn()
            .ok()?;
        let mut input = child.stdin.take()?;
        let dot_string = self.dot_string.clone().into_bytes();
        // Written while the output is read, so that neither pipe fills up and blocks `dot`.
        let writer = std::thread::spawn(move || input.write_all(&dot_string));
        let mut result = Vec::new();
        let read = output.read_to_end(&mut result);
        let written = writer.join();
        child.wait().ok()?;
        (read.is_ok() && matches!(written, Ok(Ok(())))).then_some(result)
    }
}

/// `searchDotExe`.
fn search_dot_exe(getenv_graphviz_dot: Option<String>) -> Option<PathBuf> {
    match getenv_graphviz_dot {
        Some(dot_exe) => Some(PathBuf::from(dot_exe)),
        None => find_executable_on_path(exe_name()).or_else(specific_dot_exe),
    }
}

fn exe_name() -> &'static str {
    if cfg!(windows) { "dot.exe" } else { "dot" }
}

fn find_executable_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(name))
        .find(|file| file.is_file() && can_execute(file))
        .and_then(|file| std::path::absolute(file).ok())
}

/// `GraphvizLinux.specificDotExe`, which macOS uses too: the first usable of the usual places, or else the
/// place `MacPorts` would use.
#[cfg(not(windows))]
#[allow(
    clippy::unnecessary_wraps,
    reason = "as on Windows, which may find none"
)]
fn specific_dot_exe() -> Option<PathBuf> {
    let found = [
        "/usr/local/bin/dot",
        "/opt/homebrew/bin/dot",
        "/opt/homebrew/opt/graphviz/bin/dot",
        "/usr/bin/dot",
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|file| file.exists() && can_read(file) && can_execute(file));
    Some(found.unwrap_or_else(|| PathBuf::from("/opt/local/bin/dot")))
}

/// `GraphvizWindowsOld.specificDotExeSlow`: in the first folder of `c:/` that has some, the latest of its
/// `graphviz*/bin/dot.exe` and `graphviz*/release/bin/dot.exe`. PlantUML then unpacks a `dot` it carries,
/// which rockuml does not.
#[cfg(windows)]
fn specific_dot_exe() -> Option<PathBuf> {
    sorted_entries(Path::new("c:/"))
        .into_iter()
        .filter(|folder| folder.is_dir() && can_read(folder))
        .find_map(|folder| search_in_dir(&folder))
}

#[cfg(windows)]
fn search_in_dir(folder: &Path) -> Option<PathBuf> {
    let mut dots: Vec<PathBuf> = sorted_entries(folder)
        .into_iter()
        .filter(|entry| {
            entry.is_dir()
                && entry.file_name().is_some_and(|name| {
                    name.to_string_lossy()
                        .to_lowercase()
                        .starts_with("graphviz")
                })
        })
        .flat_map(|graphviz| {
            [
                graphviz.join("bin/dot.exe"),
                graphviz.join("release/bin/dot.exe"),
            ]
        })
        .filter(|dot| dot.exists() && can_read(dot))
        .collect();
    // `higherVersion`: Java's `File` ordering, which ignores case on Windows, reversed.
    dots.sort_by_key(|dot| dot.to_string_lossy().to_lowercase());
    dots.pop()
}

#[cfg(windows)]
fn sorted_entries(folder: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    entries
}

fn can_read(path: &Path) -> bool {
    if path.is_dir() {
        fs::read_dir(path).is_ok()
    } else {
        fs::File::open(path).is_ok()
    }
}

#[cfg(unix)]
fn can_execute(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
}

/// Java finds every file executable on Windows.
#[cfg(not(unix))]
fn can_execute(path: &Path) -> bool {
    path.exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_named_dot_that_does_not_exist_is_no_installation() {
        let graphviz =
            AbstractGraphviz::new(Some("no/such/dot".to_owned()), "digraph G {\n}\n", "svg");
        assert!(!graphviz.exe_state_is_ok());
    }

    #[test]
    fn a_folder_is_no_installation() {
        let folder = tempfile::tempdir().unwrap();
        let named = folder.path().to_string_lossy().into_owned();
        assert!(!AbstractGraphviz::new(Some(named), "", "svg").exe_state_is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn dot_gets_the_source_and_its_errors_join_its_output() {
        use std::os::unix::fs::PermissionsExt;
        let folder = tempfile::tempdir().unwrap();
        let dot = folder.path().join("dot");
        fs::write(&dot, "#!/bin/sh\necho \"format $1\" >&2\ncat\n").unwrap();
        fs::set_permissions(&dot, fs::Permissions::from_mode(0o755)).unwrap();
        let graphviz = AbstractGraphviz::new(
            Some(dot.to_string_lossy().into_owned()),
            "digraph G {\n}\n",
            "svg",
        );
        assert!(graphviz.exe_state_is_ok());
        assert_eq!(
            graphviz.create_file3().as_deref(),
            Some(&b"format -Tsvg\ndigraph G {\n}\n"[..])
        );
    }
}
