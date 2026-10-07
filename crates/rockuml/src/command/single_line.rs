use super::{BlocLines, Command, CommandControl, CommandError, CommandResult};
use crate::pattern::{RegexResult, RegexTree};
use crate::text::{LineLocation, StringLocated};

/// A command that fits on one line, described by its pattern (PlantUML's `SingleLineCommand2`).
pub(crate) trait SingleLineCommand<D> {
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
}

/// A single-line command made of a pattern and what to do with what it matched.
pub(crate) struct PatternCommand<F> {
    pattern: RegexTree,
    apply: F,
}

impl<F> PatternCommand<F> {
    pub(crate) fn new(pattern: RegexTree, apply: F) -> Self {
        Self { pattern, apply }
    }
}

impl<D, F> SingleLineCommand<D> for PatternCommand<F>
where
    F: Fn(&mut D, &LineLocation, &RegexResult) -> CommandResult,
{
    fn pattern(&self) -> &RegexTree {
        &self.pattern
    }

    fn execute_arg(
        &self,
        diagram: &mut D,
        location: &LineLocation,
        arg: &RegexResult,
    ) -> CommandResult {
        (self.apply)(diagram, location, arg)
    }
}

/// Adapts a [`SingleLineCommand`] to [`Command`].
pub(crate) struct SingleLine<C>(pub C);

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
}

impl<D, C: SingleLineCommand<D>> Command<D> for SingleLine<C> {
    fn is_valid(&self, lines: &BlocLines) -> CommandControl {
        let (Some(first), 1) = (lines.first(), lines.len()) else {
            return CommandControl::NotOk;
        };
        if self.0.pattern().is_match(self.trim(first).text()) {
            CommandControl::Ok
        } else {
            CommandControl::NotOk
        }
    }

    fn execute(&self, diagram: &mut D, lines: BlocLines) -> CommandResult {
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
                RegexTree::named(1, "TITLE", "(.*)"),
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
    }

    fn validity(lines: &[&str]) -> CommandControl {
        Command::<Vec<String>>::is_valid(&SingleLine(Title::new()), &BlocLines::from_texts(lines))
    }

    #[test]
    fn exactly_one_trimmed_line_must_match() {
        assert_eq!(validity(&["  title Hello  "]), CommandControl::Ok);
        assert_eq!(
            validity(&["title Hello", "title World"]),
            CommandControl::NotOk
        );
        assert_eq!(validity(&["nope"]), CommandControl::NotOk);
    }

    #[test]
    fn executing_hands_the_named_groups_to_the_command() {
        let mut titles = Vec::new();
        let title = SingleLine(Title::new());
        title
            .execute(&mut titles, BlocLines::from_texts(&["title Hello"]))
            .unwrap();
        title
            .execute(&mut titles, BlocLines::from_texts(&[" title World "]))
            .unwrap();
        assert_eq!(titles, ["Hello", "World"]);
    }
}
