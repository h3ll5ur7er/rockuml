use crate::java;
use crate::text::{StringLocated, ends_with_backslash};

/// A diagram's preprocessed lines, from its `@start` line to its `@end` line (PlantUML's `UmlSource`).
pub struct UmlSource {
    lines: Vec<StringLocated>,
    /// The diagram as written, before preprocessing.
    raw_lines: Vec<String>,
}

impl UmlSource {
    pub fn new(lines: Vec<StringLocated>, raw_lines: Vec<String>) -> Self {
        Self { lines, raw_lines }
    }

    /// For `@startuml` diagrams, a line ending with a single backslash continues on the next line.
    pub fn with_continuations_joined(lines: &[StringLocated], raw_lines: Vec<String>) -> Self {
        let mut joined = Vec::with_capacity(lines.len());
        let mut pending = String::new();
        for line in lines {
            let text = line.text();
            if ends_with_backslash(text) {
                pending.push_str(&text[..text.len() - 1]);
            } else {
                pending.push_str(text);
                joined.push(line.with_text(std::mem::take(&mut pending)));
            }
        }
        Self {
            lines: joined,
            raw_lines,
        }
    }

    /// The lines as PlantUML encodes them into a URL: each followed by `\n`.
    pub fn plain_string(&self) -> String {
        self.lines
            .iter()
            .flat_map(|line| [line.text(), "\n"])
            .collect()
    }

    /// What PlantUML embeds into images to recover the source: the raw source, the preprocessed one when it
    /// differs, and the version.
    pub fn metadata(&self) -> String {
        let raw: String = self
            .raw_lines
            .iter()
            .flat_map(|line| [line.as_str(), "\n"])
            .collect();
        let plain = self.plain_string();
        let version = crate::PLANTUML_VERSION;
        if raw == plain {
            format!("{raw}\n{version}")
        } else {
            format!("{raw}\n{plain}\n{version}")
        }
    }

    pub fn lines(&self) -> &[StringLocated] {
        &self.lines
    }

    /// Seeds everything random in the drawing, so that the same source always gives the same image.
    pub fn seed(&self) -> i64 {
        self.lines
            .iter()
            .fold(1_125_899_906_842_597_i64, |hash, line| {
                let hash = hash
                    .wrapping_mul(31)
                    .wrapping_add(i64::from(java::string_hash_code(line.text())));
                hash.wrapping_mul(31).wrapping_add(i64::from(b'\n'))
            })
    }

    /// Drops empty, `skinparam` and `!pragma` lines right after the start line, which diagrams without
    /// commands have no use for.
    #[must_use]
    pub fn without_initial_noise(self) -> Self {
        let mut lines = self.lines;
        let noise = lines
            .iter()
            .skip(1)
            .take_while(|line| is_noise(line.text()))
            .count();
        lines.drain(1..=noise);
        Self { lines, ..self }
    }
}

fn is_noise(line: &str) -> bool {
    line.is_empty()
        || line.starts_with("skinparam ")
        || line.starts_with("skinparamlocked ")
        || line.starts_with("!pragma ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::LineLocation;

    fn source(lines: &[&str]) -> UmlSource {
        let location = LineLocation::new("test", None);
        UmlSource::new(
            lines
                .iter()
                .map(|line| StringLocated::new(*line, location.clone()))
                .collect(),
            lines.iter().map(|line| (*line).to_owned()).collect(),
        )
    }

    fn texts(source: &UmlSource) -> Vec<&str> {
        source.lines().iter().map(StringLocated::text).collect()
    }

    /// The seed PlantUML printed for `tests/corpus/creole/plain.puml`.
    #[test]
    fn the_seed_hashes_every_line() {
        let plain = source(&[
            "@startcreole",
            "Hello world",
            "This is a second line",
            "@endcreole",
        ]);
        assert_eq!(plain.seed(), 2_574_378_694_402_089_019);
    }

    #[test]
    fn continuations_are_joined_onto_the_last_line() {
        let lines = source(&["@startuml", "a \\", "b \\", "c", "d \\\\", "@enduml"]);
        let joined = UmlSource::with_continuations_joined(lines.lines(), Vec::new());
        assert_eq!(texts(&joined), ["@startuml", "a b c", "d \\\\", "@enduml"]);
    }

    #[test]
    fn initial_noise_is_removed_up_to_the_first_real_line() {
        let noisy = source(&[
            "@startcreole",
            "",
            "skinparam x y",
            "!pragma a b",
            "text",
            "",
            "@endcreole",
        ]);
        assert_eq!(
            texts(&noisy.without_initial_noise()),
            ["@startcreole", "text", "", "@endcreole"]
        );
    }
}
