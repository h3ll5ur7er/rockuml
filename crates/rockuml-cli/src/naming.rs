//! Output file names, following PlantUML: the input name with the format's suffix, numbered from the second
//! unnamed diagram on, unless the diagram names itself after `@startuml`.

use std::sync::LazyLock;

use regex::Regex;

pub struct OutputNamer {
    input_file_name: String,
    suffix: &'static str,
    unnamed_count: usize,
}

impl OutputNamer {
    pub fn new(input_file_name: &str, suffix: &'static str) -> Self {
        Self {
            input_file_name: input_file_name.to_owned(),
            suffix,
            unnamed_count: 0,
        }
    }

    pub fn next_name(&mut self, name_from_diagram: Option<&str>) -> String {
        if let Some(name) = name_from_diagram {
            return change_extension(name, self.suffix, 0);
        }
        let name = change_extension(&self.input_file_name, self.suffix, self.unnamed_count);
        self.unnamed_count += 1;
        name
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
        assert_eq!(namer.next_name(None), "flow.svg");
        assert_eq!(namer.next_name(Some("named")), "named.svg");
        assert_eq!(namer.next_name(None), "flow_001.svg");
    }

    #[test]
    fn names_without_extension_get_the_suffix_appended() {
        assert_eq!(change_extension("README", ".svg", 0), "README.svg");
        assert_eq!(change_extension("a.b.txt", ".svg", 2), "a.b_002.svg");
    }
}
