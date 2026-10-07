use regex::Regex;

use super::{BlocLines, Command, CommandControl, CommandResult, ParserPass};
use crate::pattern::RegexTree;
use crate::text::StringLocated;

/// How the first line is recognised.
enum Start {
    /// Matched against the whole trimmed line.
    Regex(Regex),
    /// A command pattern, which may use lookarounds; it anchors itself.
    Tree(&'static RegexTree),
    OwnedTree(RegexTree),
}

impl Start {
    fn is_match(&self, line: &str) -> bool {
        match self {
            Start::Regex(regex) => regex.is_match(line),
            Start::Tree(tree) => tree.is_match(line),
            Start::OwnedTree(tree) => tree.is_match(line),
        }
    }
}

/// How the last line is recognised.
enum End {
    /// Matched against the whole trimmed line.
    Regex(Regex),
    /// A command pattern, matched anywhere in the line, trimmed or not (`CommandMultilines3`).
    Tree { tree: RegexTree, trimmed: bool },
}

impl End {
    fn whole_line(end: &Regex) -> Self {
        End::Regex(whole_line(end))
    }

    fn is_match(&self, line: &StringLocated) -> bool {
        match self {
            End::Regex(regex) => regex.is_match(line.trimmed().text()),
            End::Tree {
                tree,
                trimmed: true,
            } => tree.is_match(line.trimmed().text()),
            End::Tree {
                tree,
                trimmed: false,
            } => tree.is_match(line.text()),
        }
    }
}

type Apply<D> = Box<dyn Fn(&mut D, &BlocLines) -> CommandResult>;

/// A command spanning lines from one matching `start` to one matching `end` (PlantUML's
/// `CommandMultilines`, `CommandMultilines2` and `CommandMultilines3`).
pub(crate) struct Multiline<D> {
    start: Start,
    end: End,
    /// Lines starting with a quote are comments to drop first (`MultilinesStrategy.REMOVE_STARTING_QUOTE`).
    skip_quote_lines: bool,
    /// The first line may end with `{` or have it alone on the next line.
    final_bracket: bool,
    /// Whether the block is complete once its last line matches.
    final_verification: fn(&BlocLines) -> CommandControl,
    passes: &'static [ParserPass],
    apply: Apply<D>,
}

impl<D> Multiline<D> {
    /// Both patterns must match whole trimmed lines.
    pub(crate) fn new(
        start: &Regex,
        end: &Regex,
        apply: impl Fn(&mut D, &BlocLines) -> CommandResult + 'static,
    ) -> Self {
        Self::with(Start::Regex(whole_line(start)), End::whole_line(end), apply)
    }

    /// A block whose first line matches a command pattern.
    pub(crate) fn starting_with(
        start: &'static RegexTree,
        end: &Regex,
        apply: impl Fn(&mut D, &BlocLines) -> CommandResult + 'static,
    ) -> Self {
        Self::with(Start::Tree(start), End::whole_line(end), apply)
    }

    pub(crate) fn starting_with_owned(
        start: RegexTree,
        end: &Regex,
        apply: impl Fn(&mut D, &BlocLines) -> CommandResult + 'static,
    ) -> Self {
        Self::with(Start::OwnedTree(start), End::whole_line(end), apply)
    }

    /// The block ends at a line `end` matches anywhere, once trimmed if `trimmed` (`CommandMultilines3`).
    pub(crate) fn between_trees(
        start: RegexTree,
        end: RegexTree,
        trimmed: bool,
        apply: impl Fn(&mut D, &BlocLines) -> CommandResult + 'static,
    ) -> Self {
        Self::with(
            Start::OwnedTree(start),
            End::Tree { tree: end, trimmed },
            apply,
        )
    }

    fn with(
        start: Start,
        end: End,
        apply: impl Fn(&mut D, &BlocLines) -> CommandResult + 'static,
    ) -> Self {
        Self {
            start,
            end,
            skip_quote_lines: false,
            final_bracket: false,
            final_verification: |_| CommandControl::Ok,
            passes: &[ParserPass::One],
            apply: Box::new(apply),
        }
    }

    #[must_use]
    pub(crate) fn skipping_quote_lines(self) -> Self {
        Self {
            skip_quote_lines: true,
            ..self
        }
    }

    #[must_use]
    pub(crate) fn with_final_bracket(self) -> Self {
        Self {
            final_bracket: true,
            ..self
        }
    }

    #[must_use]
    pub(crate) fn verified_by(self, final_verification: fn(&BlocLines) -> CommandControl) -> Self {
        Self {
            final_verification,
            ..self
        }
    }

    #[must_use]
    pub(crate) fn in_passes(self, passes: &'static [ParserPass]) -> Self {
        Self { passes, ..self }
    }

    fn without_comments(&self, lines: &BlocLines) -> BlocLines {
        if self.skip_quote_lines {
            lines.without_quote_lines()
        } else {
            lines.clone()
        }
    }

    fn cleaned(&self, lines: &BlocLines) -> BlocLines {
        let lines = self.without_comments(lines);
        if self.final_bracket {
            lines.eventually_move_bracket()
        } else {
            lines
        }
    }
}

/// Java matches these patterns with `matches()`, against the whole line.
fn whole_line(pattern: &Regex) -> Regex {
    Regex::new(&format!("^(?:{})$", pattern.as_str())).expect("an anchored valid pattern is valid")
}

impl<D> Command<D> for Multiline<D> {
    fn is_valid(&self, lines: &BlocLines) -> CommandControl {
        if self.final_bracket
            && let without_comments = self.without_comments(lines)
            && let (Some(first), 1) = (without_comments.first(), without_comments.len())
            && !first.trimmed().text().ends_with('{')
        {
            return match self.is_valid(&BlocLines::single(first.append(" {"))) {
                CommandControl::OkPartial => CommandControl::OkPartial,
                _ => CommandControl::NotOk,
            };
        }
        let lines = self.cleaned(lines);
        let Some(first) = lines.first() else {
            return CommandControl::NotOk;
        };
        if !self.start.is_match(first.trimmed().text()) {
            return CommandControl::NotOk;
        }
        match lines.last() {
            Some(last) if lines.len() > 1 && self.end.is_match(last) => {
                (self.final_verification)(&lines)
            }
            _ => CommandControl::OkPartial,
        }
    }

    fn execute(&self, diagram: &mut D, lines: BlocLines) -> CommandResult {
        (self.apply)(diagram, &self.cleaned(&lines))
    }

    fn is_eligible_for(&self, pass: ParserPass) -> bool {
        self.passes.contains(&pass)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::plantuml_regex;

    fn class_block() -> Multiline<()> {
        let start = RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf("class"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "NAME", r"(\w+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{"),
            RegexTree::end(),
        ]);
        Multiline::starting_with_owned(start, &plantuml_regex(r"^\}$"), |(), _| Ok(()))
            .skipping_quote_lines()
            .with_final_bracket()
    }

    fn validity(lines: &[&str]) -> CommandControl {
        class_block().is_valid(&BlocLines::from_texts(lines))
    }

    #[test]
    fn a_block_may_open_with_its_bracket_on_the_second_line() {
        assert_eq!(validity(&["class A {"]), CommandControl::OkPartial);
        assert_eq!(validity(&["class A"]), CommandControl::OkPartial);
        assert_eq!(validity(&["class A", "{"]), CommandControl::OkPartial);
        assert_eq!(
            validity(&["class A", "{", "' note", "}"]),
            CommandControl::Ok
        );
        assert_eq!(validity(&["class A {", "}"]), CommandControl::Ok);
        assert_eq!(validity(&["class A", "x"]), CommandControl::NotOk);
        assert_eq!(validity(&["interface A"]), CommandControl::NotOk);
    }
}
