//! Turns source text into preprocessed diagram blocks.

mod block;
mod reader;
mod start_utils;

use std::sync::LazyLock;

use regex::Regex;

use crate::java;
use crate::pattern::plantuml_regex;
use crate::text::StringLocated;

/// One diagram's source after preprocessing, from its `@start` line to its `@end` line.
pub struct PreprocessedBlock {
    lines: Vec<StringLocated>,
}

impl PreprocessedBlock {
    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.lines.iter().map(StringLocated::text)
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

/// Splits `source` into diagram blocks and preprocesses each. `description` names the source in errors.
pub fn preprocess(source: &str, description: &str) -> Vec<PreprocessedBlock> {
    block::extract_blocks(source, description, Vec::new())
        .into_iter()
        .map(|lines| PreprocessedBlock { lines })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output_name(source: &str) -> Option<String> {
        preprocess(source, "t").remove(0).output_name()
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
