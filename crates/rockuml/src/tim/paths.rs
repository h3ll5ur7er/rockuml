//! Resolves `!include`d names to files on disk or in the standard library, relative to the including file.

use std::path::{Component, Path, PathBuf};

use crate::host::Host;
use crate::java::RuntimeException;
use crate::stdlib::Stdlib;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Folder {
    Regular(PathBuf),
    /// PlantUML treats an included library file's own path as its folder, so this is that path.
    Stdlib {
        library: String,
        path: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum InputFile {
    Local(PathBuf),
    Stdlib { library: String, path: String },
}

impl InputFile {
    pub(super) fn read(&self, host: &dyn Host) -> Option<Vec<u8>> {
        match self {
            InputFile::Local(path) => host.read_file(path),
            InputFile::Stdlib { library, path } => {
                let file = path.to_lowercase().replace(".puml", "");
                Stdlib::retrieve(library)?.puml_resource(&file)
            }
        }
    }

    pub(super) fn parent_folder(&self) -> Folder {
        match self {
            InputFile::Local(path) => {
                Folder::Regular(path.parent().map(Path::to_path_buf).unwrap_or_default())
            }
            InputFile::Stdlib { library, path } => Folder::Stdlib {
                library: library.clone(),
                path: path.clone(),
            },
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct PathSystem {
    current: Folder,
}

impl PathSystem {
    pub(super) fn new(current: Folder) -> Self {
        Self { current }
    }

    pub(super) fn current_folder(&self) -> &Folder {
        &self.current
    }

    /// Fails for URLs, which rockuml does not fetch.
    /// URLs are not files here: where PlantUML resolves one, it fails right after (it cannot take the
    /// folder of a URL), so they report a runtime failure.
    pub(super) fn input_file(
        &self,
        name: &str,
        host: &dyn Host,
    ) -> Result<Option<InputFile>, RuntimeException> {
        if name.starts_with("http://") || name.starts_with("https://") {
            return Err(RuntimeException);
        }
        if let Some(inner) = name
            .strip_prefix('<')
            .and_then(|rest| rest.strip_suffix('>'))
        {
            let full = inner.to_lowercase();
            let (library, path) = full.split_once('/').ok_or(RuntimeException)?;
            let library = Stdlib::retrieve(library).map(|library| library.name().to_owned());
            return Ok(library.map(|library| InputFile::Stdlib {
                library,
                path: path.to_owned(),
            }));
        }
        if let Some(relative) = name.strip_prefix("::") {
            let relative = relative.strip_prefix('/').unwrap_or(relative);
            return Ok(existing(
                host,
                normalize(&host.current_directory().join(relative)),
            ));
        }
        if let Some(relative) = name.strip_prefix("~/") {
            let Some(home) = host.home_directory() else {
                return Ok(None);
            };
            return Ok(existing(host, normalize(&home.join(relative))));
        }
        Ok(match &self.current {
            Folder::Regular(directory) => {
                let path = Path::new(name);
                let path = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    directory.join(path)
                };
                existing(host, path)
            }
            Folder::Stdlib { library, path } => Some(InputFile::Stdlib {
                library: library.clone(),
                path: normalize_slashes(&format!("{path}/{name}")),
            }),
        })
    }
}

fn existing(host: &dyn Host, path: PathBuf) -> Option<InputFile> {
    host.file_exists(&path).then_some(InputFile::Local(path))
}

fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                result.pop();
            }
            Component::CurDir => {}
            other => result.push(other),
        }
    }
    result
}

fn normalize_slashes(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::IsolatedHost;

    #[test]
    fn stdlib_names_resolve_case_insensitively() {
        let paths = PathSystem::new(Folder::Regular(PathBuf::new()));
        let file = paths
            .input_file("<C4/C4_Container>", &IsolatedHost)
            .unwrap();
        assert_eq!(
            file,
            Some(InputFile::Stdlib {
                library: "c4".into(),
                path: "c4_container".into()
            })
        );
        assert!(file.unwrap().read(&IsolatedHost).is_some());
    }

    #[test]
    fn urls_are_not_files() {
        let paths = PathSystem::new(Folder::Regular(PathBuf::new()));
        assert_eq!(
            paths.input_file("https://example.com/x.puml", &IsolatedHost),
            Err(RuntimeException)
        );
    }

    #[test]
    fn dot_segments_are_collapsed() {
        assert_eq!(normalize_slashes("a/./b/../c"), "a/c");
        assert_eq!(normalize(Path::new("/x/y/../z")), PathBuf::from("/x/z"));
    }
}
