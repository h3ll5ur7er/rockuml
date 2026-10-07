//! Commands PlantUML has and rockuml has not ported yet. They accept exactly the lines PlantUML's commands
//! accept, so that the diagram type a source is read as, and the errors it gets, come out as in PlantUML;
//! executing one leaves the diagram not ported.

use regex::Regex;

use super::{
    BlocLines, Command, CommandControl, CommandResult, Multiline, ParserPass, SingleLine,
    SingleLineCommand,
};
use crate::pattern::{RegexResult, RegexTree, java_regex};
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
    forbidden: Option<Regex>,
    final_bracket: bool,
}

pub(crate) fn single_line(name: &'static str, pattern: RegexTree) -> Unported {
    Unported {
        name,
        pattern,
        forbidden: None,
        final_bracket: false,
    }
}

impl Unported {
    /// Lines matching `forbidden` as a whole are accepted, and fail as syntax errors when executed.
    #[must_use]
    pub(crate) fn forbidding(self, forbidden: &str) -> Self {
        Self {
            forbidden: Some(java_regex(&format!("^(?:{forbidden})$"), false)),
            ..self
        }
    }

    #[must_use]
    pub(crate) fn with_final_bracket(self) -> Self {
        Self {
            final_bracket: true,
            ..self
        }
    }

    pub(crate) fn boxed<D: NotPortedCommands>(self) -> Box<dyn Command<D>> {
        Box::new(SingleLine(self))
    }

    /// Also accepted over up to `max_lines` further lines, joined with hidden newlines
    /// (`CommandDecoratorMultine`).
    pub(crate) fn over_lines<D: NotPortedCommands>(self, max_lines: usize) -> Box<dyn Command<D>> {
        Box::new(DecoratorMultine {
            command: SingleLine(self),
            max_lines,
        })
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
        self.forbidden
            .as_ref()
            .is_some_and(|forbidden| forbidden.is_match(line))
    }

    fn syntax_with_final_bracket(&self) -> bool {
        self.final_bracket
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

/// A multi-line command whose last line is recognised by a command pattern (`CommandMultilines3`).
pub(crate) fn multi_line3<D: NotPortedCommands + 'static>(
    name: &'static str,
    start: RegexTree,
    end: RegexTree,
    trimmed: bool,
) -> Multiline<D> {
    Multiline::between_trees(
        start,
        end,
        trimmed,
        move |diagram: &mut D, _: &BlocLines| {
            diagram.command_not_ported(name);
            Ok(())
        },
    )
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

struct DecoratorMultine<C> {
    command: SingleLine<C>,
    max_lines: usize,
}

impl<D, C: SingleLineCommand<D>> Command<D> for DecoratorMultine<C> {
    fn is_valid(&self, lines: &BlocLines) -> CommandControl {
        let joined = lines.to_single_line_with_hidden_new_line();
        if self
            .command
            .0
            .is_forbidden(joined.first().map_or("", |line| line.text()))
        {
            return CommandControl::NotOk;
        }
        match self.command.is_valid(&joined) {
            CommandControl::Ok => CommandControl::Ok,
            _ => CommandControl::OkPartial,
        }
    }

    fn execute(&self, diagram: &mut D, lines: BlocLines) -> CommandResult {
        self.command
            .execute(diagram, lines.to_single_line_with_hidden_new_line())
    }

    fn is_eligible_for(&self, pass: ParserPass) -> bool {
        self.command.0.is_eligible_for(pass)
    }

    fn max_lines(&self) -> Option<usize> {
        Some(self.max_lines)
    }
}
