//! The common commands rockuml has not ported yet.

use crate::command::Command;
use crate::command::unported::{self, NotPortedCommands};
use crate::pattern::RegexTree;

/// PlantUML's `CommandNamespaceSeparator`.
pub(super) fn namespace_separator<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandNamespaceSeparator",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"set"),
            RegexTree::spaces_one_or_more(),
            RegexTree::or(vec![
                RegexTree::leaf(r"separator"),
                RegexTree::leaf(r"namespaceseparator"),
            ]),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(
                1,
                "SEPARATOR",
                r"((?:none|null)|[\\]{2}|::|[^%pLN%s_$#\\{}<>%g])",
            ),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandAssumeTransparent`.
pub(super) fn assume_transparent<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandAssumeTransparent",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"!assume"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"transparent"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "TYPE", r"(dark|light)"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandSkin`.
pub(super) fn skin<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandSkin",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"skin"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "SKIN", r"([\w.]+)"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandMinwidth`.
pub(super) fn minwidth<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandMinwidth",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"minwidth"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "VALUE", r"(\d+)"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandPage`.
pub(super) fn page<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandPage",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"page"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "NB1", r"(\d+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"x*"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "NB2", r"(\d+)"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandRotate`.
pub(super) fn rotate<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandRotate",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"rotate"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandStyleSingleLineCSS`.
pub(super) fn style_single_line_css<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandStyleSingleLineCSS",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"\<style\>"),
            RegexTree::named(1, "STYLE", r"([-#.\w%s;:{}]+)"),
            RegexTree::leaf(r"\</style\>"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandStyleImport`.
pub(super) fn style_import<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandStyleImport",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"\<style"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\w+"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"="),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"[%q%g]?"),
            RegexTree::named(1, "PATH", r"([^%q%g]*)"),
            RegexTree::leaf(r"[%q%g]?"),
            RegexTree::leaf(r"\>"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandHideEmptyDescription`.
pub(super) fn hide_empty_description<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandHideEmptyDescription",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "HIDE", r"(hide|show)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"empty"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"description"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandHideShowByVisibility`.
pub(super) fn hide_show_by_visibility<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandHideShowByVisibility",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "COMMAND", r"(hide|show)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "VISIBILITY", r"((?:public|private|protected|package)?(?:[,%s]+(?:public|private|protected|package))*)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "PORTION", r"(members?|attributes?|fields?|methods?)"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandHideShowByGender`.
pub(super) fn hide_show_by_gender<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandHideShowByGender",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "COMMAND", r"(hide|show)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "GENDER", r"(?:(class|object|interface|enum|annotation|dataclass|record|abstract|[%pLN_.]+|[%g][^%g]+[%g]|\<\<.*\>\>)[%s]+)*?"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "EMPTY", r"(empty)"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(1, "PORTION", r"(members?|attributes?|fields?|methods?|circles?|circled?|stereotypes?)"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}
