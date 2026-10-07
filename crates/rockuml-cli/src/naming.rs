//! Output file names, following PlantUML: the input name with the format's suffix, numbered from the second
//! unnamed diagram on, unless the diagram names itself after `@startuml`.

use std::sync::LazyLock;

use regex::Regex;

pub(crate) struct OutputNamer {
    input_file_name: String,
    suffix: &'static str,
    unnamed_count: usize,
}

impl OutputNamer {
    pub(crate) fn new(input_file_name: &str, suffix: &'static str) -> Self {
        Self {
            input_file_name: input_file_name.to_owned(),
            suffix,
            unnamed_count: 0,
        }
    }

    /// The names of a diagram's pages; the pages after the first take the numbers of the diagrams that
    /// follow.
    pub(crate) fn next_names(
        &mut self,
        name_from_diagram: Option<&str>,
        pages: usize,
    ) -> Vec<String> {
        if let Some(name) = name_from_diagram {
            return (0..pages)
                .map(|page| change_extension(name, self.suffix, page))
                .collect();
        }
        let names = (0..pages)
            .map(|page| {
                change_extension(
                    &self.input_file_name,
                    self.suffix,
                    self.unnamed_count + page,
                )
            })
            .collect();
        self.unnamed_count += pages;
        names
    }
}

fn change_extension(file_name: &str, suffix: &str, counter: usize) -> String {
    static EXTENSION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\.[a-zA-Z0-9_]+$").unwrap());

    let replacement = if counter == 0 {
        suffix.to_owned()
    } else {
        format!("_{counter:03}{suffix}")
    };
    if EXTENSION.is_match(file_name) {
        EXTENSION
            .replace(file_name, replacement.as_str())
            .into_owned()
    } else {
        format!("{file_name}{replacement}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unnamed_diagrams_are_numbered_from_the_second_on() {
        let mut namer = OutputNamer::new("flow.puml", ".svg");
        assert_eq!(namer.next_names(None, 1), ["flow.svg"]);
        assert_eq!(namer.next_names(Some("named"), 1), ["named.svg"]);
        assert_eq!(namer.next_names(None, 1), ["flow_001.svg"]);
    }

    #[test]
    fn pages_take_the_numbers_of_following_diagrams() {
        let mut namer = OutputNamer::new("flow.puml", ".svg");
        assert_eq!(namer.next_names(None, 2), ["flow.svg", "flow_001.svg"]);
        assert_eq!(namer.next_names(None, 1), ["flow_002.svg"]);
    }

    #[test]
    fn names_without_extension_get_the_suffix_appended() {
        assert_eq!(change_extension("README", ".svg", 0), "README.svg");
        assert_eq!(change_extension("a.b.txt", ".svg", 2), "a.b_002.svg");
    }
}
