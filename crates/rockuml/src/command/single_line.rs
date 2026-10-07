use super::{BlocLines, Command, CommandControl, CommandError, CommandResult, ParserPass};
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

    /// A line the pattern accepts but the command refuses: executing it is a syntax error.
    fn is_forbidden(&self, _line: &str) -> bool {
        false
    }

    /// Whether the line ends with `{`, which may also stand alone on the next line.
    fn syntax_with_final_bracket(&self) -> bool {
        false
    }

    fn is_eligible_for(&self, pass: ParserPass) -> bool {
        pass == ParserPass::One
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

/// The line with ` {` appended, as if the bracket on the next line were on it.
fn with_final_bracket(line: &StringLocated) -> BlocLines {
    BlocLines::single(line.append(" {"))
}

impl<C> SingleLine<C> {
    /// The line, then `{` alone on the next one.
    fn is_valid_bracket<D>(&self, lines: &BlocLines) -> CommandControl
    where
        C: SingleLineCommand<D>,
    {
        let mut lines = lines.iter();
        let (Some(first), Some(second)) = (lines.next(), lines.next()) else {
            unreachable!("two lines were counted");
        };
        if self.trim(second).text() != "{" {
            return CommandControl::NotOk;
        }
        Command::<D>::is_valid(self, &with_final_bracket(first))
    }
}

impl<D, C: SingleLineCommand<D>> Command<D> for SingleLine<C> {
    fn is_valid(&self, lines: &BlocLines) -> CommandControl {
        let final_bracket = self.0.syntax_with_final_bracket();
        if lines.len() == 2 && final_bracket {
            return self.is_valid_bracket::<D>(lines);
        }
        let (Some(first), 1) = (lines.first(), lines.len()) else {
            return CommandControl::NotOk;
        };
        let line = self.trim(first);
        if final_bracket && !line.text().ends_with('{') {
            return match self.is_valid(&with_final_bracket(first)) {
                CommandControl::Ok => CommandControl::OkPartial,
                _ => CommandControl::NotOk,
            };
        }
        if self.0.pattern().is_match(line.text()) {
            CommandControl::Ok
        } else {
            CommandControl::NotOk
        }
    }

    fn execute(&self, diagram: &mut D, lines: BlocLines) -> CommandResult {
        let lines = match (lines.first(), lines.len()) {
            (Some(first), 2) if self.0.syntax_with_final_bracket() => with_final_bracket(first),
            _ => lines,
        };
        let (Some(first), 1) = (lines.first(), lines.len()) else {
            panic!("a single-line command executes exactly one line, got {lines:?}");
        };
        let line = self.trim(first);
        if self.0.is_forbidden(line.text()) {
            return Err(CommandError::new(format!("Syntax error: {}", line.text())));
        }
        let Some(arg) = self.0.pattern().matcher(line.text()) else {
            return Err(CommandError::new(format!(
                "Cannot parse line {}",
                line.text()
            )));
        };
        self.0.execute_arg(diagram, first.location(), &arg)
    }

    fn is_eligible_for(&self, pass: ParserPass) -> bool {
        self.0.is_eligible_for(pass)
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

    /// `package name {`, whose bracket may stand alone on the next line.
    struct Package(RegexTree);

    impl SingleLineCommand<Vec<String>> for Package {
        fn pattern(&self) -> &RegexTree {
            &self.0
        }

        fn execute_arg(
            &self,
            names: &mut Vec<String>,
            _: &LineLocation,
            arg: &RegexResult,
        ) -> CommandResult {
            names.push(arg.get("NAME", 0).unwrap_or_default().to_owned());
            Ok(())
        }

        fn is_forbidden(&self, line: &str) -> bool {
            line.contains("forbidden")
        }

        fn syntax_with_final_bracket(&self) -> bool {
            true
        }
    }

    fn package() -> SingleLine<Package> {
        SingleLine(Package(RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf("package"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "NAME", r"(\w+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{"),
            RegexTree::end(),
        ])))
    }

    #[test]
    fn a_final_bracket_may_come_on_the_next_line() {
        let validity = |lines: &[&str]| {
            Command::<Vec<String>>::is_valid(&package(), &BlocLines::from_texts(lines))
        };
        assert_eq!(validity(&["package a {"]), CommandControl::Ok);
        assert_eq!(validity(&["package a"]), CommandControl::OkPartial);
        assert_eq!(validity(&["package a", " { "]), CommandControl::Ok);
        assert_eq!(validity(&["package a", "class B"]), CommandControl::NotOk);
        assert_eq!(validity(&["package"]), CommandControl::NotOk);
        let mut names = Vec::new();
        package()
            .execute(&mut names, BlocLines::from_texts(&["package a", "{"]))
            .unwrap();
        assert_eq!(names, ["a"]);
    }

    #[test]
    fn forbidden_lines_are_accepted_and_fail_when_executed() {
        let lines = BlocLines::from_texts(&["package forbidden {"]);
        assert_eq!(
            Command::<Vec<String>>::is_valid(&package(), &lines),
            CommandControl::Ok
        );
        let error = package().execute(&mut Vec::new(), lines).unwrap_err();
        assert_eq!(error.message, "Syntax error: package forbidden {");
    }
}
