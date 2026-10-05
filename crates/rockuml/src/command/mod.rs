//! Diagram commands: each recognises some source lines and applies them to the diagram being built.

mod bloc_lines;
pub mod factory;
mod multiline;
mod single_line;

pub use bloc_lines::BlocLines;
pub use multiline::Multiline;
pub use single_line::{SingleLine, SingleLineCommand};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandControl {
    Ok,
    NotOk,
    /// The lines so far start the command but more are needed to complete it.
    OkPartial,
}

/// Some diagrams read their source more than once, e.g. to declare everything before linking.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParserPass {
    One,
    Two,
    Three,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandError {
    pub message: String,
    /// Ranks errors when several diagram types fail, to report the most plausible one.
    pub score: i32,
}

impl CommandError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            score: 0,
        }
    }

    pub fn no_such_color() -> Self {
        Self {
            message: "No such color".to_owned(),
            score: 10,
        }
    }
}

pub type CommandResult = Result<(), CommandError>;

pub trait Command<D> {
    fn is_valid(&self, lines: &BlocLines) -> CommandControl;

    fn execute(&self, diagram: &mut D, lines: BlocLines, pass: ParserPass) -> CommandResult;

    fn is_eligible_for(&self, pass: ParserPass) -> bool {
        pass == ParserPass::One
    }
}
