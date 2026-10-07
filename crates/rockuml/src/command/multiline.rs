use regex::Regex;

use super::{BlocLines, Command, CommandControl, CommandResult};
use crate::pattern::RegexTree;

/// How the first line is recognised.
enum Start {
    Regex(Regex),
    /// A command pattern, which may use lookarounds; it anchors itself.
    Tree(&'static RegexTree),
}

impl Start {
    fn is_match(&self, line: &str) -> bool {
        match self {
            Start::Regex(regex) => regex.is_match(line),
            Start::Tree(tree) => tree.is_match(line),
        }
    }
}

/// A command spanning lines from one matching `start` to one matching `end` (PlantUML's
/// `CommandMultilines` and `CommandMultilines2`). Both patterns must match whole trimmed lines.
pub(crate) struct Multiline<D> {
    start: Start,
    end: Regex,
    /// Lines starting with a quote are comments to drop first.
    skip_quote_lines: bool,
    apply: fn(&mut D, &BlocLines) -> CommandResult,
}

impl<D> Multiline<D> {
    pub(crate) fn new(
        start: &Regex,
        end: &Regex,
        apply: fn(&mut D, &BlocLines) -> CommandResult,
    ) -> Self {
        Self {
            start: Start::Regex(whole_line(start)),
            end: whole_line(end),
            skip_quote_lines: false,
            apply,
        }
    }

    /// A block whose first line matches a command pattern.
    pub(crate) fn starting_with(
        start: &'static RegexTree,
        end: &Regex,
        apply: fn(&mut D, &BlocLines) -> CommandResult,
    ) -> Self {
        Self {
            start: Start::Tree(start),
            end: whole_line(end),
            skip_quote_lines: false,
            apply,
        }
    }

    #[must_use]
    pub(crate) fn skipping_quote_lines(self) -> Self {
        Self {
            skip_quote_lines: true,
            ..self
        }
    }

    fn cleaned(&self, lines: &BlocLines) -> BlocLines {
        if self.skip_quote_lines {
            lines.without_quote_lines()
        } else {
            lines.clone()
        }
    }
}

/// Java matches these patterns with `matches()`, against the whole line.
fn whole_line(pattern: &Regex) -> Regex {
    Regex::new(&format!("^(?:{})$", pattern.as_str())).expect("an anchored valid pattern is valid")
}

impl<D> Command<D> for Multiline<D> {
    fn is_valid(&self, lines: &BlocLines) -> CommandControl {
        let lines = self.cleaned(lines);
        let Some(first) = lines.first() else {
            return CommandControl::NotOk;
        };
        if !self.start.is_match(first.trimmed().text()) {
            return CommandControl::NotOk;
        }
        match lines.last() {
            Some(last) if lines.len() > 1 && self.end.is_match(last.trimmed().text()) => {
                CommandControl::Ok
            }
            _ => CommandControl::OkPartial,
        }
    }

    fn execute(&self, diagram: &mut D, lines: BlocLines) -> CommandResult {
        (self.apply)(diagram, &self.cleaned(&lines))
    }
}
