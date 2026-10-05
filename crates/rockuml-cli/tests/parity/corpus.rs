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
    fn stem(&self) -> &str {
        self.source
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap()
    }

    /// PlantUML names page N of a multi-page diagram `stem_00N.ext`; the first page is `stem.ext`.
    pub fn goldens(&self, kind: GoldenKind) -> BTreeMap<String, PathBuf> {
        let directory = self.source.parent().unwrap();
        fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| is_output_of(path, self.stem(), kind))
            .map(|path| (file_name(&path), path))
            .collect()
    }
}

pub fn is_output_of(path: &Path, stem: &str, kind: GoldenKind) -> bool {
    if path.extension().and_then(|extension| extension.to_str()) != Some(kind.extension()) {
        return false;
    }
    let Some(output_stem) = path
        .file_stem()
        .and_then(|output_stem| output_stem.to_str())
    else {
        return false;
    };
    match output_stem.strip_prefix(stem) {
        Some("") => true,
        Some(suffix) => {
            suffix.len() == 4
                && suffix.starts_with('_')
                && suffix[1..].bytes().all(|byte| byte.is_ascii_digit())
        }
        None => false,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_first_and_subsequent_pages_only() {
        let is_debug_output = |name: &str| is_output_of(Path::new(name), "flow", GoldenKind::Debug);

        assert!(is_debug_output("flow.debug"));
        assert!(is_debug_output("flow_001.debug"));
        assert!(!is_debug_output("flow.svg"));
        assert!(!is_debug_output("flowchart.debug"));
        assert!(!is_debug_output("flow_01.debug"));
        assert!(!is_debug_output("flow_abc.debug"));
    }
}
