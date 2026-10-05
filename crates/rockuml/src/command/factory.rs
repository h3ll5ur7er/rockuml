//! Feeds a diagram's lines to its commands (PlantUML's `PSystemCommandFactory`).

use super::{BlocLines, Command, CommandControl, CommandError, ParserPass};
use crate::preproc::start_utils;
use crate::text::StringLocated;

/// Why the lines did not make a diagram: no command accepts a line ("Syntax Error?"), or a command could
/// not apply the lines it accepted.
#[derive(Clone, Debug, PartialEq)]
pub struct ParseFailure {
    pub error: CommandError,
    /// The lines read up to the failure, the faulty one last.
    pub trace: Vec<StringLocated>,
}

/// Runs the lines after the start line through `commands`, up to the end line.
pub fn execute_lines<D>(
    lines: &[StringLocated],
    diagram: &mut D,
    commands: &[Box<dyn Command<D>>],
) -> Result<(), ParseFailure> {
    let mut position = 1;
    while let Some(line) = lines.get(position) {
        if start_utils::is_end_directive(line.text()) {
            break;
        }
        let Some((command, block, next)) = candidate(lines, position, commands) else {
            return Err(ParseFailure {
                error: CommandError::new("Syntax Error?"),
                trace: lines[..=position].to_vec(),
            });
        };
        position = next;
        if !command.is_eligible_for(ParserPass::One) {
            continue;
        }
        command
            .execute(diagram, block, ParserPass::One)
            .map_err(|error| ParseFailure {
                error,
                trace: lines[..position].to_vec(),
            })?;
    }
    Ok(())
}

/// The first command that accepts the line at `position`, the lines it takes and where parsing goes on.
fn candidate<'a, D>(
    lines: &[StringLocated],
    position: usize,
    commands: &'a [Box<dyn Command<D>>],
) -> Option<(&'a dyn Command<D>, BlocLines, usize)> {
    let single = BlocLines::single(lines[position].clone());
    for command in commands {
        match command.is_valid(&single) {
            CommandControl::Ok => return Some((command.as_ref(), single, position + 1)),
            CommandControl::OkPartial => {
                if let Some((block, next)) = multiline_block(lines, position, command.as_ref()) {
                    return Some((command.as_ref(), block, next));
                }
            }
            CommandControl::NotOk => {}
        }
    }
    None
}

/// Adds lines until the command accepts the block, or gives up on it.
fn multiline_block<D>(
    lines: &[StringLocated],
    position: usize,
    command: &dyn Command<D>,
) -> Option<(BlocLines, usize)> {
    let mut block = BlocLines::default();
    for (index, line) in lines.iter().enumerate().skip(position) {
        block = block.add(line.clone());
        match command.is_valid(&block) {
            CommandControl::NotOk => return None,
            CommandControl::Ok => return Some((block, index + 1)),
            CommandControl::OkPartial => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{CommandResult, SingleLine, SingleLineCommand};
    use crate::pattern::{RegexResult, RegexTree};
    use crate::text::LineLocation;

    struct Word(RegexTree);

    impl SingleLineCommand<Vec<String>> for Word {
        fn pattern(&self) -> &RegexTree {
            &self.0
        }

        fn execute_arg(
            &self,
            words: &mut Vec<String>,
            _: &LineLocation,
            arg: &RegexResult,
            _: ParserPass,
        ) -> CommandResult {
            words.push(arg.get("WORD", 0).unwrap_or_default().to_owned());
            Ok(())
        }
    }

    fn word_commands() -> Vec<Box<dyn Command<Vec<String>>>> {
        let pattern = RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "WORD", "([a-z]+)"),
            RegexTree::end(),
        ]);
        vec![Box::new(SingleLine(Word(pattern)))]
    }

    fn lines(texts: &[&str]) -> Vec<StringLocated> {
        let location = LineLocation::new("test", None);
        texts
            .iter()
            .map(|text| StringLocated::new(*text, location.clone()))
            .collect()
    }

    #[test]
    fn lines_run_through_commands_until_the_end_line() {
        let mut words = Vec::new();
        let source = lines(&["@startx", "one", "two", "@endx", "three"]);
        execute_lines(&source, &mut words, &word_commands()).unwrap();
        assert_eq!(words, ["one", "two"]);
    }

    #[test]
    fn a_line_no_command_takes_is_a_syntax_error() {
        let mut words = Vec::new();
        let source = lines(&["@startx", "one", "2", "@endx"]);
        let failure = execute_lines(&source, &mut words, &word_commands()).unwrap_err();
        assert_eq!(failure.error.message, "Syntax Error?");
        assert_eq!(failure.trace.last().map(StringLocated::text), Some("2"));
    }
}
