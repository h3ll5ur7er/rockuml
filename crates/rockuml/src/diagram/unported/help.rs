//! The commands of PlantUML's help diagrams (`HelpFactory`).

use crate::command::Command;
use crate::command::unported::{self, NotPortedCommands};
use crate::pattern::RegexTree;

/// PlantUML's `CommandHelpColor`.
pub(super) fn help_color<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandHelpColor",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"help"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"colors?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandHelpFont`.
pub(super) fn help_font<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandHelpFont",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"help"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"fonts?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandHelpKeyword`.
pub(super) fn help_keyword<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandHelpKeyword",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"help"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"keywords?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandHelpType`.
pub(super) fn help_type<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandHelpType",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"help"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"types?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandHelpTheme`.
pub(super) fn help_theme<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandHelpTheme",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"help"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"themes?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}
