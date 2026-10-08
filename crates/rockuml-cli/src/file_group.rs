//! The input files an argument names (PlantUML's `FileGroup`): a file, the diagram files of a directory, or
//! the files a pattern with `*`, `?` or `**` matches.

use std::cmp::Ordering;
use std::fs;
use std::path::{MAIN_SEPARATOR, Path, PathBuf};
use std::sync::LazyLock;

use regex::{Regex, RegexBuilder};

/// The files of a directory argument that PlantUML reads (`CliOptions.getPattern`).
const DIAGRAM_FILE_NAMES: &str = r"(?i)^.*\.(txt|tex|java|htm|html|c|h|cpp|apt|pu|puml|hpp|hh)$";

/// The files `pattern` names, sorted, without those an `excluded` pattern matches.
pub(crate) fn files(pattern: &str, excluded: &[String]) -> Vec<PathBuf> {
    let mut group = FileGroup {
        result: Vec::new(),
        excluded: excluded
            .iter()
            .filter_map(|exclude| regex(&to_regexp(exclude)))
            .collect(),
    };
    if !pattern.contains(['*', '?']) {
        group.init_no_star(pattern);
    } else if pattern.contains("**") {
        group.recurse(pattern);
    } else {
        group.init_with_simple_star(pattern);
    }
    group
        .result
        .sort_by(|left, right| compare_paths(left, right));
    group.result
}

struct FileGroup {
    result: Vec<PathBuf>,
    excluded: Vec<Regex>,
}

impl FileGroup {
    fn init_no_star(&mut self, pattern: &str) {
        let file = java_file(pattern);
        if file.is_dir() {
            self.add_simple_directory(&file, &DIAGRAM_FILES);
        } else if file.is_file() {
            self.add_result_file(file);
        }
    }

    fn init_with_simple_star(&mut self, pattern: &str) {
        static NO_STAR_IN_DIRECTORY: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^(?:([^*?]*)[/\\])?([^/\\]*)$").unwrap());
        let Some(captures) = NO_STAR_IN_DIRECTORY.captures(pattern) else {
            self.recurse(pattern);
            return;
        };
        let directory = captures
            .get(1)
            .map_or(PathBuf::from("."), |part| java_file(part.as_str()));
        if let Some(files_part) = regex(&to_regexp(&captures[2])) {
            self.add_simple_directory(&directory, &files_part);
        }
    }

    fn recurse(&mut self, pattern: &str) {
        static PREDIR_PATH: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^([^*?]*[/\\])?(.*)$").unwrap());
        let parent = PREDIR_PATH
            .captures(pattern)
            .and_then(|captures| captures.get(1))
            .map_or(PathBuf::from("."), |part| java_file(part.as_str()));
        if let Some(matcher) = regex(&to_regexp(pattern)) {
            self.init_with_double_star(&parent, &matcher);
        }
    }

    /// Unlike PlantUML, which overflows its stack on a link cycle, links to directories are not followed.
    fn init_with_double_star(&mut self, directory: &Path, matcher: &Regex) {
        for path in entries(directory) {
            if path.is_dir() && !path.is_symlink() {
                self.init_with_double_star(&path, matcher);
            } else if path.is_file() && matcher.is_match(&normalized_path(&path)) {
                self.add_result_file(path);
            }
        }
    }

    /// Unlike PlantUML, which takes matching subdirectories for files too, only files are taken.
    fn add_simple_directory(&mut self, directory: &Path, file_names: &Regex) {
        for path in entries(directory) {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned());
            if path.is_file() && name.is_some_and(|name| file_names.is_match(&name)) {
                self.add_result_file(path);
            }
        }
    }

    fn add_result_file(&mut self, file: PathBuf) {
        let path = normalized_path(&file);
        if !self
            .excluded
            .iter()
            .any(|excluded| excluded.is_match(&path))
        {
            self.result.push(file);
        }
    }
}

static DIAGRAM_FILES: LazyLock<Regex> = LazyLock::new(|| Regex::new(DIAGRAM_FILE_NAMES).unwrap());

fn entries(directory: &Path) -> Vec<PathBuf> {
    fs::read_dir(directory)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .collect()
        })
        .unwrap_or_default()
}

/// A wildcard pattern as the regular expression PlantUML matches paths written with `/` against: `?` is
/// one character and `*` any characters within a directory, `**` any characters across directories.
pub(crate) fn to_regexp(pattern: &str) -> String {
    let pattern = pattern
        .replace('\\', "/")
        .replace('.', r"\.")
        .replace('?', "[^/]")
        .replace("/**/", "(/|/.{0,}/)")
        .replace("**", ".{0,}")
        .replace('*', "[^/]{0,}");
    format!(r"(?i)^(\./)?{pattern}$")
}

/// PlantUML takes the rest of the pattern for a regular expression too; one rockuml cannot compile
/// matches nothing.
fn regex(expression: &str) -> Option<Regex> {
    RegexBuilder::new(expression).build().ok()
}

/// The path as Java's `File` holds it: platform separators, no repeated or trailing ones.
pub(crate) fn java_file(path: &str) -> PathBuf {
    let mut result = String::with_capacity(path.len());
    for c in path.chars() {
        let c = if c == '/' || c == '\\' && MAIN_SEPARATOR == '\\' {
            MAIN_SEPARATOR
        } else {
            c
        };
        if !(c == MAIN_SEPARATOR && result.ends_with(MAIN_SEPARATOR)) {
            result.push(c);
        }
    }
    if result.len() > 1
        && result.ends_with(MAIN_SEPARATOR)
        && !result.ends_with(&format!(":{MAIN_SEPARATOR}"))
    {
        result.pop();
    }
    PathBuf::from(result)
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Java sorts files by path, ignoring case on Windows.
fn compare_paths(left: &Path, right: &Path) -> Ordering {
    let (left, right) = (left.to_string_lossy(), right.to_string_lossy());
    if cfg!(windows) {
        let fold = |c: char| c.to_uppercase().flat_map(char::to_lowercase);
        left.chars()
            .flat_map(fold)
            .cmp(right.chars().flat_map(fold))
    } else {
        left.encode_utf16().cmp(right.encode_utf16())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patterns_become_regular_expressions() {
        assert_eq!(to_regexp("proj/**.puml"), r"(?i)^(\./)?proj/.{0,}\.puml$");
        assert_eq!(to_regexp("**/*.puml"), r"(?i)^(\./)?.{0,}/[^/]{0,}\.puml$");
        assert_eq!(to_regexp(r"a\b?/**/c"), r"(?i)^(\./)?a/b[^/](/|/.{0,}/)c$");
    }

    #[test]
    fn paths_are_held_like_java_files() {
        let separator = MAIN_SEPARATOR.to_string();
        assert_eq!(java_file("proj/").to_string_lossy(), "proj");
        assert_eq!(
            java_file("a//b").to_string_lossy(),
            ["a", "b"].join(&separator)
        );
        assert_eq!(java_file("/").to_string_lossy(), separator);
    }

    fn tree() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        for file in [
            "a.puml",
            "Upper.PUML",
            "x.txt",
            "notes.md",
            "sub/b.puml",
            "sub/deep/c.pu",
        ] {
            let path = root.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "").unwrap();
        }
        root
    }

    fn names(root: &Path, pattern: &str, excluded: &[&str]) -> Vec<String> {
        let excluded: Vec<String> = excluded.iter().map(|&exclude| exclude.to_owned()).collect();
        let root = normalized_path(root);
        files(&format!("{root}/{pattern}"), &excluded)
            .iter()
            .map(|file| normalized_path(file)[root.len() + 1..].to_owned())
            .collect()
    }

    #[test]
    fn directories_give_their_diagram_files() {
        let root = tree();
        let mut found = names(root.path(), "", &[]);
        found.sort();
        assert_eq!(found, ["Upper.PUML", "a.puml", "x.txt"]);
    }

    #[test]
    fn stars_match_within_a_directory_and_double_stars_across() {
        let root = tree();
        assert_eq!(names(root.path(), "s?b/*.puml", &[]), ["sub/b.puml"]);
        assert_eq!(names(root.path(), "**/deep/*", &[]), ["sub/deep/c.pu"]);
        let mut found = names(root.path(), "**.puml", &["**/sub/**"]);
        found.sort();
        assert_eq!(found, ["Upper.PUML", "a.puml"]);
    }
}
