//! The common commands rockuml has not ported yet.

use crate::command::Command;
use crate::command::unported::{self, NotPortedCommands};
use crate::pattern::RegexTree;

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
