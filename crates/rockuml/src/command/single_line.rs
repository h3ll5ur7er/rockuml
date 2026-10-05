use super::{BlocLines, Command, CommandControl, CommandError, CommandResult};
use crate::pattern::{RegexResult, RegexTree};
use crate::text::{LineLocation, StringLocated};

/// A command that fits on one line, described by its pattern (PlantUML's `SingleLineCommand2`).
pub trait SingleLineCommand<D> {
    fn pattern(&self) -> &RegexTree;

    fn execute_arg(
        &self,
        diagram: &mut D,
        location: &LineLocation,
        arg: &RegexResult,
    ) -> CommandResult;

    fn trims_line(&self) -> bool {
        true
    }

    /// Whether the line ends with `{`, which may also be written alone on the next line.
    fn syntax_with_final_bracket(&self) -> bool {
        false
    }

    fn final_verification(&self) -> CommandControl {
        CommandControl::Ok
    }
}

/// Adapts a [`SingleLineCommand`] to [`Command`].
pub struct SingleLine<C>(pub C);

impl<C> SingleLine<C> {
    fn trim<D>(&self, line: &StringLocated) -> StringLocated
    where
        C: SingleLineCommand<D>,
    {
        if self.0.trims_line() {
            line.trimmed()
        } else {
            line.clone()
        }
    }

    fn with_bracket_joined<D>(&self, lines: BlocLines) -> BlocLines
    where
        C: SingleLineCommand<D>,
    {
        match (lines.first(), lines.len()) {
            (Some(first), 2) if self.0.syntax_with_final_bracket() => {
                BlocLines::single(first.append(" {"))
            }
            _ => lines,
        }
    }

    fn is_valid_bracket<D>(&self, lines: &BlocLines) -> CommandControl
    where
        C: SingleLineCommand<D>,
    {
        let (Some(first), Some(second)) = (lines.first(), lines.get(1)) else {
            return CommandControl::NotOk;
        };
        if self.trim(second).text() != "{" {
            return CommandControl::NotOk;
        }
        self.is_valid(&BlocLines::single(first.append(" {")))
    }
}

impl<D, C: SingleLineCommand<D>> Command<D> for SingleLine<C> {
    fn is_valid(&self, lines: &BlocLines) -> CommandControl {
        if lines.len() == 2 && self.0.syntax_with_final_bracket() {
            return self.is_valid_bracket(lines);
        }
        let (Some(first), 1) = (lines.first(), lines.len()) else {
            return CommandControl::NotOk;
        };
        let line = self.trim(first);
        if self.0.syntax_with_final_bracket() && !line.text().ends_with('{') {
            let with_bracket = BlocLines::single(first.append(" {"));
            return match self.is_valid(&with_bracket) {
                CommandControl::Ok => CommandControl::OkPartial,
                _ => CommandControl::NotOk,
            };
        }
        if self.0.pattern().is_match(line.text()) {
            self.0.final_verification()
        } else {
            CommandControl::NotOk
        }
    }

    fn execute(&self, diagram: &mut D, lines: BlocLines) -> CommandResult {
        let lines = self.with_bracket_joined(lines);
        let (Some(first), 1) = (lines.first(), lines.len()) else {
            panic!("a single-line command executes exactly one line, got {lines:?}");
        };
        let line = self.trim(first);
        let Some(arg) = self.0.pattern().matcher(line.text()) else {
            return Err(CommandError::new(format!(
                "Cannot parse line {}",
                line.text()
            )));
        };
        self.0.execute_arg(diagram, first.location(), &arg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Title(RegexTree);

    impl Title {
        fn new() -> Self {
            Self(RegexTree::concat(vec![
                RegexTree::start(),
                RegexTree::leaf("title"),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(1, "TITLE", "(.*?)"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::leaf("\\{"),
                RegexTree::end(),
            ]))
        }
    }

    impl SingleLineCommand<Vec<String>> for Title {
        fn pattern(&self) -> &RegexTree {
            &self.0
        }

        fn execute_arg(
            &self,
            titles: &mut Vec<String>,
            _: &LineLocation,
            arg: &RegexResult,
        ) -> CommandResult {
            titles.push(arg.get("TITLE", 0).unwrap_or_default().to_owned());
            Ok(())
        }

        fn syntax_with_final_bracket(&self) -> bool {
            true
        }
    }

    fn validity(lines: &[&str]) -> CommandControl {
        Command::<Vec<String>>::is_valid(&SingleLine(Title::new()), &BlocLines::from_texts(lines))
    }

    #[test]
    fn the_final_bracket_may_be_on_the_line_or_the_next() {
        assert_eq!(validity(&["  title Hello {"]), CommandControl::Ok);
        assert_eq!(validity(&["title Hello"]), CommandControl::OkPartial);
        assert_eq!(validity(&["title Hello", " { "]), CommandControl::Ok);
        assert_eq!(validity(&["title Hello", "x"]), CommandControl::NotOk);
        assert_eq!(validity(&["nope"]), CommandControl::NotOk);
    }

    #[test]
    fn executing_hands_the_named_groups_to_the_command() {
        let mut titles = Vec::new();
        let title = SingleLine(Title::new());
        title
            .execute(&mut titles, BlocLines::from_texts(&["title Hello", "{"]))
            .unwrap();
        title
            .execute(&mut titles, BlocLines::from_texts(&["title World {"]))
            .unwrap();
        assert_eq!(titles, ["Hello", "World"]);
    }
}
