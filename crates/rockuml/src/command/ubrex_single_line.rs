use super::{BlocLines, Command, CommandControl, CommandResult};
use crate::text::LineLocation;
use crate::ubrex::{UMatcher, UnicodeBracketedExpression};

/// A one-line command matched whole by a Unicode bracketed expression, and what to do with what it
/// captured (PlantUML's `UBrexSingleLineCommand2`).
pub(crate) struct UBrexSingleLine<F> {
    pattern: UnicodeBracketedExpression,
    apply: F,
}

impl<F> UBrexSingleLine<F> {
    pub(crate) fn new(pattern: UnicodeBracketedExpression, apply: F) -> Self {
        Self { pattern, apply }
    }
}

impl<D, F> Command<D> for UBrexSingleLine<F>
where
    F: Fn(&mut D, &LineLocation, &UMatcher) -> CommandResult,
{
    fn is_valid(&self, lines: &BlocLines) -> CommandControl {
        match (lines.first(), lines.len()) {
            (Some(first), 1) if self.pattern.exact_match(first.trimmed().text()) => {
                CommandControl::Ok
            }
            _ => CommandControl::NotOk,
        }
    }

    fn execute(&self, diagram: &mut D, lines: BlocLines) -> CommandResult {
        let line = lines.first().expect("a command has its line").trimmed();
        let matcher = self
            .pattern
            .match_at(line.text())
            .expect("the line matched when the command was chosen");
        (self.apply)(diagram, line.location(), &matcher)
    }
}
