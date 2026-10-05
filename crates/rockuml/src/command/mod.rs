//! Diagram commands: each recognises some source lines and applies them to the diagram being built.

mod bloc_lines;
pub(crate) mod factory;
mod multiline;
mod single_line;

pub(crate) use bloc_lines::BlocLines;
pub(crate) use multiline::Multiline;
pub(crate) use single_line::{SingleLine, SingleLineCommand};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommandControl {
    Ok,
    NotOk,
    /// The lines so far start the command but more are needed to complete it.
    OkPartial,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommandError {
    pub message: String,
}

impl CommandError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub(crate) type CommandResult = Result<(), CommandError>;

pub(crate) trait Command<D> {
    fn is_valid(&self, lines: &BlocLines) -> CommandControl;

    fn execute(&self, diagram: &mut D, lines: BlocLines) -> CommandResult;
}
