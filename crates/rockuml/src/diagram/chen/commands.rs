//! The commands of Chen entity relationship diagrams, in the order `ChenEerDiagramFactory` tries them.
//! None is ported yet: they only recognise their lines.

use crate::color;
use crate::command::Command;
use crate::command::unported::{self, NotPortedCommands};
use crate::pattern::RegexTree;

/// PlantUML's `CommandCreateEntity`.
pub(super) fn create_entity<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandCreateEntity",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "TYPE", r"(entity|relationship)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "DISPLAY", r"[%g]([^%g]+)[%g]"),
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"as"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(1, "CODE", r"([%pLN_.]+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "STEREO", r"(<<.+>>)?"),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandCreateAttribute`.
pub(super) fn create_attribute<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandCreateAttribute",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "DISPLAY", r"[%g]([^%g]+)[%g]"),
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"as"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(1, "CODE", r"([%pLN%s_.:]+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "STEREO", r"(<<.*>>)?"),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "COMPOSITE", r"(\{)?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandAssociate`.
pub(super) fn associate<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandAssociate",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "NAME1", r"([%pLN_.-]+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "PARTICIPATION", r"([-=])"),
            RegexTree::optional(RegexTree::named(
                1,
                "CARDINALITY",
                r"([%pLN]+|\([%pLN]+,[%s]*[%pLN]+\))",
            )),
            RegexTree::named(1, "PARTICIPATION2", r"([-=])"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "NAME2", r"([%pLN_.-]+)"),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandEndGroup`.
pub(super) fn end_group<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandEndGroup",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"\}"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandSimpleSubclass`.
pub(super) fn simple_subclass<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandSimpleSubclass",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "NAME1", r"([%pLN_.-]+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "PARTICIPATION", r"([-=])"),
            RegexTree::named(1, "DIRECTION", r"([<>])"),
            RegexTree::named(1, "PARTICIPATION2", r"([-=])"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "NAME2", r"([%pLN_.-]+)"),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandMultiSubclass`.
pub(super) fn multi_subclass<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandMultiSubclass",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "SUPERCLASS", r"([%pLN_.-]+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "PARTICIPATION", r"([-=])"),
            RegexTree::counted(1, r"(>)"),
            RegexTree::named(1, "PARTICIPATION2", r"([-=])"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "SYMBOL", r"([doU])"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{"),
            RegexTree::named(1, "SUBCLASSES", r"(.+)"),
            RegexTree::leaf(r"\}"),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}
