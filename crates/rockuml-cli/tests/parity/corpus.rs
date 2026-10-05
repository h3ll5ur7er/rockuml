use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum GoldenKind {
    Preprocessed,
    Debug,
}

impl GoldenKind {
    pub const ALL: [GoldenKind; 2] = [GoldenKind::Preprocessed, GoldenKind::Debug];

    pub fn extension(self) -> &'static str {
        match self {
            GoldenKind::Preprocessed => "preproc",
            GoldenKind::Debug => "debug",
        }
    }

    pub fn cli_arguments(self) -> &'static [&'static str] {
        match self {
            GoldenKind::Preprocessed => &["-preproc"],
            GoldenKind::Debug => &["-f", "debug"],
        }
    }
}

pub struct Case {
    /// Path relative to the corpus root with `/` separators, stable across platforms.
    pub id: String,
    pub source: PathBuf,
}

impl Case {
    /// Everything the golden model wrote for this case, keyed by file name.
    pub fn goldens(&self, kind: GoldenKind) -> BTreeMap<String, PathBuf> {
        let golden_directory = self.source.with_extension("golden");
        let Ok(entries) = fs::read_dir(golden_directory) else {
            return BTreeMap::new();
        };
        entries
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == kind.extension())
            })
            .map(|path| (file_name(&path), path))
            .collect()
    }
}

pub fn file_name(path: &Path) -> String {
    path.file_name().unwrap().to_string_lossy().into_owned()
}

pub fn discover(corpus_root: &Path) -> Vec<Case> {
    let mut cases = Vec::new();
    collect_cases(corpus_root, corpus_root, &mut cases);
    cases.sort_by(|left, right| left.id.cmp(&right.id));
    cases
}

fn collect_cases(corpus_root: &Path, directory: &Path, cases: &mut Vec<Case>) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_cases(corpus_root, &path, cases);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "puml")
        {
            let id = path
                .strip_prefix(corpus_root)
                .unwrap()
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            cases.push(Case { id, source: path });
        }
    }
}
