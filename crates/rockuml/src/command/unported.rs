//! Commands PlantUML has and rockuml has not ported yet. They accept exactly the lines PlantUML's commands
//! accept, so that the diagram type a source is read as, and the errors it gets, come out as in PlantUML;
//! executing one leaves the diagram not ported.

use super::{Command, CommandResult, SingleLine, SingleLineCommand};
use crate::pattern::{RegexResult, RegexTree};
use crate::text::LineLocation;

/// A diagram told that it holds something rockuml cannot draw yet.
pub(crate) trait NotPortedCommands {
    /// `command` names the PlantUML command that accepted the lines.
    fn command_not_ported(&mut self, command: &'static str);
}

/// A single-line command (`SingleLineCommand2`) known by its Java class name and pattern.
pub(crate) struct Unported {
    name: &'static str,
    pattern: RegexTree,
}

pub(crate) fn single_line(name: &'static str, pattern: RegexTree) -> Unported {
    Unported { name, pattern }
}

impl Unported {
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
}
