//! `--help` and `--help-more` (PlantUML's `HelpPrint`, `HelpGroup` and `HelpTable`), listing the flags
//! rockuml offers.

use crate::cli_flag::{CliFlag, Support};
use crate::exit_status::ExitStatus;

/// `--help`: the flags of level 0.
pub(crate) fn help() -> String {
    help_up_to_level(0)
}

/// `--help-more`: the flags of levels 0 and 1.
pub(crate) fn help_more() -> String {
    help_up_to_level(1)
}

fn help_up_to_level(limit: i32) -> String {
    let mut text = format!(
        "rockuml - generate diagrams from plain text, like PlantUML {}\n\n{HEADER}",
        rockuml::PLANTUML_VERSION
    );
    text.push_str(&help_table(limit));
    text.push_str("\n\n");
    text.push_str(EXAMPLES);
    text.push_str("\nExit codes:\n");
    for (code, description) in ExitStatus::exit_codes() {
        text.push_str(&format!("  {code:<4} {description}\n"));
    }
    text.push_str(SEE_ALSO);
    text
}

const HEADER: &str = concat!(
    "Usage:\n  rockuml [options] [file|dir]...\n\nDescription:\n",
    "  Process PlantUML sources from files, directories (optionally recursive), or stdin (-pipe).\n\n",
    "Wildcards (for files/dirs):\n",
    "  *   any characters except '/' and '\\'\n",
    "  ?   exactly one character except '/' and '\\'\n",
    "  **  any characters across directories (recursive)\n",
    "  Tip: quote patterns to avoid shell expansion (e.g., \"**/*.puml\").\n",
);

const EXAMPLES: &str = concat!(
    "Examples:\n",
    "  # Process all .puml recursively\n",
    "  rockuml \"**/*.puml\"\n\n",
    "  # Check syntax only (CI)\n",
    "  rockuml --check-syntax src/diagrams\n\n",
    "  # Read from stdin and write to stdout (SVG)\n",
    "  cat diagram.puml | rockuml --svg -pipe > out.svg\n\n",
    "  # Encode a sprite from an image\n",
    "  rockuml --sprite 16 myicon.png\n\n",
    "  # Use a define\n",
    "  rockuml -DAUTHOR=John diagram.puml\n\n",
    "  # Change output directory\n",
    "  rockuml --format svg --output-dir out diagrams/\n",
);

const SEE_ALSO: &str = concat!(
    "\nSee also:\n",
    "  rockuml --help-more\n",
    "  Documentation: https://plantuml.com\n",
);

/// The groups of flags, each started by the documented flag that names it, then the flags rockuml offers
/// in each, sorted. Groups without such flags are left out.
fn help_table(limit: i32) -> String {
    let mut groups: Vec<(&str, Vec<CliFlag>)> = vec![("General", Vec::new())];
    for &flag in CliFlag::VALUES {
        let Some(doc) = flag.doc().filter(|doc| doc.level <= limit) else {
            continue;
        };
        if !doc.new_group.is_empty() {
            groups.push((doc.new_group, Vec::new()));
        }
        if flag.support() == Support::Ported {
            groups.last_mut().expect("General comes first").1.push(flag);
        }
    }
    let mut table = HelpTable::default();
    for (title, mut flags) in groups.into_iter().filter(|(_, flags)| !flags.is_empty()) {
        flags.sort_by_cached_key(|flag| sort_key(&flag.usage()));
        table.new_group(title);
        for flag in flags {
            table.new_line(
                flag.usage(),
                flag.doc().expect("listed flags are documented").value,
            );
        }
    }
    table.print_me()
}

/// Sorts `-v, --verbose` before `--version`: letters only, long flags led by their first letter.
fn sort_key(usage: &str) -> String {
    let only_letters: String = usage
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<String>()
        .to_lowercase();
    if usage.trim().starts_with("--") {
        let first = only_letters
            .chars()
            .next()
            .map(String::from)
            .unwrap_or_default();
        format!("{first}{only_letters}")
    } else {
        only_letters
    }
}

/// Two columns, the first padded with dots (`HelpTable`).
#[derive(Default)]
struct HelpTable {
    /// A group heading has no first column.
    lines: Vec<(String, String)>,
}

impl HelpTable {
    fn new_group(&mut self, title: &str) {
        self.lines.push((String::new(), title.to_owned()));
    }

    fn new_line(&mut self, first: String, second: &str) {
        self.lines.push((first, second.to_owned()));
    }

    fn size_first_column(&self) -> usize {
        self.lines
            .iter()
            .map(|(first, _)| first.find('\n').unwrap_or(first.len()))
            .max()
            .unwrap_or(0)
    }

    fn print_me(&self) -> String {
        let size = self.size_first_column();
        let mut out = String::new();
        for (first, second) in &self.lines {
            if first.is_empty() {
                out.push_str(&format!("\n{second}:\n"));
                continue;
            }
            let mut second_lines = second.split('\n');
            out.push_str(&format!(
                "{} {}\n",
                pad(&format!("{first} "), size + 2, '.'),
                second_lines.next().unwrap_or_default()
            ));
            for line in second_lines {
                out.push_str(&format!("{} {line}\n", pad(" ", size + 2, ' ')));
            }
        }
        out
    }
}

/// Pads `text` to `size` with `fill`, except that a text one short of it gets a space.
fn pad(text: &str, size: usize, fill: char) -> String {
    let length = text.chars().count();
    if length + 1 == size {
        return format!("{text} ");
    }
    let mut result = text.to_owned();
    result.extend(std::iter::repeat_n(fill, size.saturating_sub(length)));
    result
}

/// `--version`.
pub(crate) fn version() -> String {
    format!(
        "rockuml {} (PlantUML {} compatible)\n",
        env!("CARGO_PKG_VERSION"),
        rockuml::PLANTUML_VERSION
    )
}

/// `--author`.
pub(crate) fn author() -> String {
    format!(
        "rockuml {}: a port of PlantUML {} to Rust.\n\
         PlantUML is written by Arnaud Roques and its contributors: https://plantuml.com\n",
        env!("CARGO_PKG_VERSION"),
        rockuml::PLANTUML_VERSION
    )
}

/// `-license`.
pub(crate) fn license() -> String {
    format!(
        "rockuml ports PlantUML {}, which is free software under the GNU Lesser General Public License\n\
         (LGPL) version 3 or later: https://plantuml.com/license\n\
         The images it generates belong to the authors of their sources.\n",
        rockuml::PLANTUML_VERSION
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_padded_with_dots_to_the_widest_first_column() {
        let mut table = HelpTable::default();
        table.new_group("General");
        table.new_line(" -h, --help".to_owned(), "Show help");
        table.new_line("     --version".to_owned(), "Show version\nand more");
        assert_eq!(
            table.print_me(),
            "\nGeneral:\n -h, --help .... Show help\n     --version   Show version\n                 and more\n"
        );
    }

    #[test]
    fn short_flags_sort_with_their_long_names() {
        let mut usages = ["     --version", " -v, --verbose", "     --author"];
        usages.sort_by_cached_key(|usage| sort_key(usage));
        assert_eq!(
            usages,
            ["     --author", " -v, --verbose", "     --version"]
        );
    }

    #[test]
    fn help_lists_only_what_rockuml_offers() {
        let help = help();
        assert!(help.contains(" -h, --help "));
        assert!(help.contains("     --svg "));
        assert!(!help.contains("--eps"));
        assert!(!help.contains("--gui"));
        assert!(!help.contains("--charset"));
        assert!(help_more().contains("     --charset <name> "));
        assert!(!help.contains("java -jar"));
    }
}
