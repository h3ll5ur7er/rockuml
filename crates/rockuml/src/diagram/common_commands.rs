//! Commands every diagram with a skin understands (PlantUML's `CommonCommands`).

use super::titled::TitledDiagram;
use crate::command::{Command, CommandResult, ParserPass, SingleLine, SingleLineCommand};
use crate::creole::Display;
use crate::pattern::{RegexResult, RegexTree};
use crate::text::LineLocation;

/// `title My diagram`, or `title: "My diagram"`.
pub fn title_command<D: TitledDiagram + 'static>() -> Box<dyn Command<D>> {
    Box::new(SingleLine(Title(RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::leaf("title"),
        RegexTree::leaf("(?:[%s]*:[%s]*|[%s]+)"),
        RegexTree::or(vec![
            RegexTree::named(1, "TITLE1", "[%g](.*)[%g]"),
            RegexTree::named(1, "TITLE2", "(.*[%pLN_.].*)"),
        ]),
        RegexTree::end(),
    ]))))
}

struct Title(RegexTree);

impl<D: TitledDiagram> SingleLineCommand<D> for Title {
    fn pattern(&self) -> &RegexTree {
        &self.0
    }

    fn execute_arg(
        &self,
        diagram: &mut D,
        _: &LineLocation,
        arg: &RegexResult,
        _: ParserPass,
    ) -> CommandResult {
        let title = arg.get_lazzy("TITLE", 0).unwrap_or_default();
        diagram.titled().set_title(Display::with_newlines(title));
        Ok(())
    }
}
