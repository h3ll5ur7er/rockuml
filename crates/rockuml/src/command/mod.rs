//! Diagram commands: each recognises some source lines and applies them to the diagram being built.

mod bloc_lines;
pub(crate) mod factory;
mod multiline;
mod single_line;
pub(crate) mod unported;

pub(crate) use bloc_lines::BlocLines;
pub(crate) use multiline::Multiline;
pub(crate) use single_line::{PatternCommand, SingleLine, SingleLineCommand};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommandControl {
    Ok,
    NotOk,
    /// The lines so far start the command but more are needed to complete it.
    OkPartial,
}

/// The rounds in which a diagram reads its lines. State diagrams read them three times, so that links and
/// notes can name states declared further down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParserPass {
    One,
    Two,
    Three,
}

impl ParserPass {
    /// Just this pass, for a command that runs in one pass.
    pub(crate) fn alone(self) -> &'static [ParserPass] {
        match self {
            Self::One => &[Self::One],
            Self::Two => &[Self::Two],
            Self::Three => &[Self::Three],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommandError {
    pub message: String,
    /// Weighs the error against the errors of the other diagram types the lines were tried as.
    pub score: i32,
}

impl CommandError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self::with_score(message, 0)
    }

    pub(crate) fn with_score(message: impl Into<String>, score: i32) -> Self {
        Self {
            message: message.into(),
            score,
        }
    }

    /// A colour name no colour has (`CommandExecutionResult.badColor`).
    pub(crate) fn bad_color() -> Self {
        Self::with_score("No such color", 10)
    }
}

pub(crate) type CommandResult = Result<(), CommandError>;

pub(crate) trait Command<D> {
    fn is_valid(&self, lines: &BlocLines) -> CommandControl;

    fn execute(&self, diagram: &mut D, lines: BlocLines) -> CommandResult;

    /// Whether the command runs in `pass`. In other passes the lines it accepts are skipped.
    fn is_eligible_for(&self, pass: ParserPass) -> bool {
        pass == ParserPass::One
    }

    /// How many lines past the first a block may grow to before the command gives up on it.
    fn max_lines(&self) -> Option<usize> {
        None
    }
}
