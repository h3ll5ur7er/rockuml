//! Feeds a diagram's lines to its commands (PlantUML's `PSystemCommandFactory`).

use super::{BlocLines, Command, CommandControl, CommandError, ParserPass};
use crate::diagram::UmlSource;
use crate::preproc::start_utils;
use crate::text::StringLocated;

/// Why the lines did not make a diagram: no command accepts a line ("Syntax Error?"), a command could
/// not apply the lines it accepted, or the diagram they built is wrong as a whole.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ParseFailure {
    pub error: CommandError,
    /// The lines read up to the failure, the faulty one last.
    pub trace: Vec<StringLocated>,
}

impl ParseFailure {
    /// How far the lines got, to pick the error of the diagram type they most likely are
    /// (`PSystemError.score`).
    pub(crate) fn score(&self) -> usize {
        self.trace.len() * 10 + self.error.score as usize
    }
}

/// What the factory asks of the diagram it builds, besides running its commands (PlantUML's
/// `AbstractDiagram` hooks).
pub(crate) trait AbstractDiagram {
    /// The passes over the lines the diagram needs.
    fn required_pass(&self) -> &'static [ParserPass] {
        &[ParserPass::One]
    }

    fn starting_pass(&mut self, _pass: ParserPass) {}

    /// The error that makes the whole diagram wrong; it may complete the diagram on the way.
    fn check_final_error(&mut self) -> Option<String> {
        None
    }

    fn make_diagram_ready(&mut self) {}

    /// Whether the lines are no such diagram after all, which is no error either.
    fn is_incomplete(&self) -> bool {
        false
    }
}

/// What a factory made of the lines.
pub(crate) enum Created<D> {
    Diagram(D),
    Failure(ParseFailure),
    /// The lines are no such diagram (PlantUML's `null`).
    Nothing,
}

/// The diagram `empty_diagram` makes of the source, run through `commands` in each pass it requires
/// (`PSystemCommandFactory.createSystem`).
pub(crate) fn create_system<D: AbstractDiagram>(
    source: &UmlSource,
    empty_diagram: impl FnOnce() -> D,
    commands: &[Box<dyn Command<D>>],
) -> Created<D> {
    let lines = source.lines();
    if source.is_empty() {
        return Created::Failure(empty_description(lines));
    }
    let mut diagram = empty_diagram();
    for &pass in diagram.required_pass() {
        diagram.starting_pass(pass);
        if let Err(failure) = execute_lines(lines, &mut diagram, commands, pass) {
            return Created::Failure(failure);
        }
    }
    finalize_diagram(diagram, lines)
}

fn finalize_diagram<D: AbstractDiagram>(mut diagram: D, lines: &[StringLocated]) -> Created<D> {
    // Each pass starts over after the start line, so errors found now point at the second line.
    let trace = || lines.iter().take(2).cloned().collect();
    if let Some(error) = diagram.check_final_error() {
        return Created::Failure(ParseFailure {
            error: CommandError::new(error),
            trace: trace(),
        });
    }
    if lines.len() == 2 {
        return Created::Failure(empty_description(lines));
    }
    diagram.make_diagram_ready();
    if diagram.is_incomplete() {
        return Created::Nothing;
    }
    Created::Diagram(diagram)
}

/// The error of a source that holds nothing, at its second line.
pub(crate) fn empty_description(lines: &[StringLocated]) -> ParseFailure {
    ParseFailure {
        error: CommandError::new("Empty description"),
        trace: lines.iter().take(2).cloned().collect(),
    }
}

/// Runs the lines after the start line through the commands of `pass`, up to the end line.
pub(crate) fn execute_lines<D>(
    lines: &[StringLocated],
    diagram: &mut D,
    commands: &[Box<dyn Command<D>>],
    pass: ParserPass,
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
        if !command.is_eligible_for(pass) {
            continue;
        }
        command
            .execute(diagram, block)
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
    for (added, (index, line)) in lines.iter().enumerate().skip(position).enumerate() {
        block = block.add(line.clone());
        match command.is_valid(&block) {
            CommandControl::NotOk => return None,
            CommandControl::Ok => return Some((block, index + 1)),
            CommandControl::OkPartial => {}
        }
        if command.max_lines().is_some_and(|max| added + 1 > max) {
            return None;
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

    #[derive(Default)]
    struct Words {
        accepted: Vec<String>,
        passes: Vec<ParserPass>,
        final_error: Option<String>,
    }

    impl AbstractDiagram for Words {
        fn required_pass(&self) -> &'static [ParserPass] {
            &[ParserPass::One, ParserPass::Two]
        }

        fn starting_pass(&mut self, pass: ParserPass) {
            self.passes.push(pass);
        }

        fn check_final_error(&mut self) -> Option<String> {
            self.final_error.clone()
        }

        fn is_incomplete(&self) -> bool {
            self.accepted.len() < 2
        }
    }

    struct Word(RegexTree, ParserPass);

    impl SingleLineCommand<Words> for Word {
        fn pattern(&self) -> &RegexTree {
            &self.0
        }

        fn execute_arg(
            &self,
            diagram: &mut Words,
            _: &LineLocation,
            arg: &RegexResult,
        ) -> CommandResult {
            let word = arg.get("WORD", 0).unwrap_or_default();
            if word == "bad" {
                return Err(CommandError::with_score("Bad word", 3));
            }
            diagram.accepted.push(format!("{word}@{:?}", self.1));
            Ok(())
        }

        fn is_eligible_for(&self, pass: ParserPass) -> bool {
            pass == self.1
        }
    }

    fn word_commands() -> Vec<Box<dyn Command<Words>>> {
        let word = |pattern: &'static str, pass| {
            Box::new(SingleLine(Word(
                RegexTree::concat(vec![
                    RegexTree::start(),
                    RegexTree::named(1, "WORD", pattern),
                    RegexTree::end(),
                ]),
                pass,
            ))) as Box<dyn Command<Words>>
        };
        vec![
            word("([a-z]+)", ParserPass::One),
            word("([0-9]+)", ParserPass::Two),
        ]
    }

    fn source(texts: &[&str]) -> UmlSource {
        let location = LineLocation::new("test", None);
        let lines = texts
            .iter()
            .map(|text| StringLocated::new(*text, location.clone()))
            .collect();
        UmlSource::new(lines, Vec::new())
    }

    fn create(texts: &[&str], final_error: Option<&str>) -> Created<Words> {
        let empty = || Words {
            final_error: final_error.map(str::to_owned),
            ..Words::default()
        };
        create_system(&source(texts), empty, &word_commands())
    }

    fn failure(created: Created<Words>) -> (String, i32, Vec<String>) {
        let Created::Failure(failure) = created else {
            panic!("expected a failure");
        };
        let trace = failure
            .trace
            .iter()
            .map(|line| line.text().to_owned())
            .collect();
        (failure.error.message, failure.error.score, trace)
    }

    #[test]
    fn each_pass_runs_the_commands_eligible_for_it_up_to_the_end_line() {
        let Created::Diagram(diagram) = create(&["@startx", "one", "22", "@endx", "three"], None)
        else {
            panic!("expected a diagram");
        };
        assert_eq!(diagram.accepted, ["one@One", "22@Two"]);
        assert_eq!(diagram.passes, [ParserPass::One, ParserPass::Two]);
    }

    #[test]
    fn a_line_no_command_takes_is_a_syntax_error_on_the_lines_read_so_far() {
        let (message, score, trace) = failure(create(&["@startx", "one", "two-", "@endx"], None));
        assert_eq!((message.as_str(), score), ("Syntax Error?", 0));
        assert_eq!(trace, ["@startx", "one", "two-"]);
    }

    #[test]
    fn a_command_error_keeps_its_score() {
        let (message, score, trace) = failure(create(&["@startx", "bad", "@endx"], None));
        assert_eq!((message.as_str(), score), ("Bad word", 3));
        assert_eq!(trace, ["@startx", "bad"]);
    }

    #[test]
    fn final_errors_and_empty_sources_point_at_the_second_line() {
        let (message, _, trace) = failure(create(&["@startx", "one", "@endx"], Some("Wrong")));
        assert_eq!(message, "Wrong");
        assert_eq!(trace, ["@startx", "one"]);
        let (message, _, trace) = failure(create(&["@startx", " ' nothing", "@endx"], None));
        assert_eq!(message, "Empty description");
        assert_eq!(trace, ["@startx", " ' nothing"]);
        let (message, _, trace) = failure(create(&["@startx", "@endx"], None));
        assert_eq!(message, "Empty description");
        assert_eq!(trace, ["@startx", "@endx"]);
    }

    #[test]
    fn an_incomplete_diagram_is_nothing() {
        assert!(matches!(
            create(&["@startx", "22", "@endx"], None),
            Created::Nothing
        ));
    }
}
