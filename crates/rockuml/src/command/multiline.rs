use regex::Regex;

use super::{BlocLines, Command, CommandControl, CommandResult};

/// A command spanning lines from one matching `start` to one matching `end` (PlantUML's
/// `CommandMultilines` and `CommandMultilines2`). Both patterns are tried on trimmed lines.
pub struct Multiline<D> {
    start: Regex,
    end: Regex,
    /// Lines starting with a quote are comments to drop first.
    skip_quote_lines: bool,
    apply: fn(&mut D, &BlocLines) -> CommandResult,
}

impl<D> Multiline<D> {
    pub fn new(start: Regex, end: Regex, apply: fn(&mut D, &BlocLines) -> CommandResult) -> Self {
        Self {
            start,
            end,
            skip_quote_lines: false,
            apply,
        }
    }

    #[must_use]
    pub fn skipping_quote_lines(self) -> Self {
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
