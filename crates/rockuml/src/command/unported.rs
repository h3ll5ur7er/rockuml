//! Commands PlantUML has and rockuml has not ported yet. They accept exactly the lines PlantUML's commands
//! accept, so that the diagram type a source is read as, and the errors it gets, come out as in PlantUML;
//! executing one leaves the diagram not ported.

use regex::Regex;

use super::{
    BlocLines, Command, CommandControl, CommandResult, Multiline, SingleLine, SingleLineCommand,
};
use crate::pattern::{RegexResult, RegexTree};
use crate::text::LineLocation;
use crate::ubrex::UnicodeBracketedExpression;

/// A diagram told that it holds something rockuml cannot draw yet.
pub(crate) trait NotPortedCommands {
    /// `command` names the PlantUML command that accepted the lines.
    fn command_not_ported(&mut self, command: &'static str);
}

/// A single-line command (`SingleLineCommand2`) known by its Java class name and pattern.
pub(crate) struct Unported {
    name: &'static str,
    pattern: RegexTree,
    forbidden: Option<fn(&str) -> bool>,
}

pub(crate) fn single_line(name: &'static str, pattern: RegexTree) -> Unported {
    Unported {
        name,
        pattern,
        forbidden: None,
    }
}

impl Unported {
    /// Lines `forbidden` holds for are accepted, and fail as syntax errors when executed.
    #[must_use]
    pub(crate) fn forbidding(self, forbidden: fn(&str) -> bool) -> Self {
        Self {
            forbidden: Some(forbidden),
            ..self
        }
    }

    pub(crate) fn boxed<D: NotPortedCommands>(self) -> Box<dyn Command<D>> {
        Box::new(SingleLine(self))
    }
}

impl<D: NotPortedCommands> SingleLineCommand<D> for Unported {
    fn pattern(&self) -> &RegexTree {
        &self.pattern
    }

    fn execute_arg(&self, diagram: &mut D, _: &LineLocation, _: &RegexResult) -> CommandResult {
        diagram.command_not_ported(self.name);
        Ok(())
    }

    fn is_forbidden(&self, line: &str) -> bool {
        self.forbidden.is_some_and(|forbidden| forbidden(line))
    }
}

/// A multi-line command (`CommandMultilines2`): from a line `start` accepts to one `end` matches whole.
pub(crate) fn multi_line<D: NotPortedCommands + 'static>(
    name: &'static str,
    start: RegexTree,
    end: &Regex,
) -> Multiline<D> {
    Multiline::starting_with_owned(start, end, move |diagram: &mut D, _: &BlocLines| {
        diagram.command_not_ported(name);
        Ok(())
    })
}

/// A single-line command recognised by a Unicode bracketed expression matching the whole trimmed line
/// (`UBrexSingleLineCommand2`).
pub(crate) fn ubrex_single_line<D: NotPortedCommands>(
    name: &'static str,
    pattern: UnicodeBracketedExpression,
) -> Box<dyn Command<D>> {
    Box::new(UnportedUBrex { name, pattern })
}

struct UnportedUBrex {
    name: &'static str,
    pattern: UnicodeBracketedExpression,
}

impl<D: NotPortedCommands> Command<D> for UnportedUBrex {
    fn is_valid(&self, lines: &BlocLines) -> CommandControl {
        match (lines.first(), lines.len()) {
            (Some(first), 1) if self.pattern.exact_match(first.trimmed().text()) => {
                CommandControl::Ok
            }
            _ => CommandControl::NotOk,
        }
    }

    fn execute(&self, diagram: &mut D, _: BlocLines) -> CommandResult {
        diagram.command_not_ported(self.name);
        Ok(())
    }
}
