//! Single-line commands whose line may be broken over several.

use super::{
    BlocLines, Command, CommandControl, CommandResult, ParserPass, SingleLine, SingleLineCommand,
};

/// A single-line command also accepted over up to `max_lines` further lines, joined with hidden newlines
/// (PlantUML's `CommandDecoratorMultine`).
pub(crate) struct DecoratorMultine<C> {
    command: SingleLine<C>,
    max_lines: usize,
}

impl<C> DecoratorMultine<C> {
    pub(crate) fn new(command: C, max_lines: usize) -> Self {
        Self {
            command: SingleLine(command),
            max_lines,
        }
    }
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
