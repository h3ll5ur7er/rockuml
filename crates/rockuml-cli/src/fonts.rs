//! Fonts the user adds to the embedded ones, from `--font` options and the `ROCKUML_FONTS` path list.

use std::fs;
use std::path::{Path, PathBuf};

use rockuml::fonts::FontRegistry;

const FONT_PATH_VARIABLE: &str = "ROCKUML_FONTS";
const FONT_EXTENSIONS: [&str; 3] = ["ttf", "otf", "ttc"];

/// The embedded fonts plus those in `ROCKUML_FONTS` and in `paths`; a directory adds every font file in it.
/// `ROCKUML_FONTS` entries that do not exist are skipped, as the variable is set once for many runs.
pub fn load(paths: &[PathBuf]) -> Result<FontRegistry, String> {
    let from_environment: Vec<PathBuf> = std::env::var_os(FONT_PATH_VARIABLE)
        .map(|list| {
            std::env::split_paths(&list)
                .filter(|path| !path.as_os_str().is_empty() && path.exists())
                .collect()
        })
        .unwrap_or_default();
    let mut registry = FontRegistry::default();
    for path in from_environment.iter().chain(paths) {
        for file in font_files(path)? {
            let data = fs::read(&file)
                .map_err(|error| format!("cannot read font {}: {error}", file.display()))?;
            registry
                .register(data)
                .map_err(|error| format!("{}: {error}", file.display()))?;
        }
    }
    Ok(registry)
}

fn font_files(path: &Path) -> Result<Vec<PathBuf>, String> {
    if !path.is_dir() {
        return Ok(vec![path.to_path_buf()]);
    }
    let entries =
        fs::read_dir(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|file| {
            file.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    FONT_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
                })
        })
        .collect();
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directories_contribute_their_font_files_in_name_order() {
        let directory = tempfile::tempdir().unwrap();
        for name in ["b.TTF", "a.otf", "notes.txt"] {
            fs::write(directory.path().join(name), b"").unwrap();
        }
        let names: Vec<_> = font_files(directory.path())
            .unwrap()
            .iter()
            .map(|file| file.file_name().unwrap().to_owned())
            .collect();
        assert_eq!(names, ["a.otf", "b.TTF"]);
    }

    #[test]
    fn files_that_are_no_fonts_are_reported() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("broken.ttf");
        fs::write(&file, b"not a font").unwrap();
        let error = load(&[file]).err().unwrap();
        assert!(error.contains("broken.ttf"), "{error}");
    }
}
