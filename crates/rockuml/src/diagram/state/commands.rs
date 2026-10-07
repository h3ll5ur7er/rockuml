//! The commands of state diagrams, in the order `StateDiagramFactory` tries them. None is ported yet:
//! they only recognise their lines.

use crate::command::unported::{self, NotPortedCommands};
use crate::command::{Command, ParserPass};
use crate::klimt::url::Url;
use crate::pattern::RegexTree;
use crate::{color, stereo};

/// PlantUML's `CommandCreateState`.
pub(super) fn create_state<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandCreateState",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"state"),
            RegexTree::spaces_one_or_more(),
            RegexTree::or(vec![
                RegexTree::concat(vec![
                    RegexTree::named(1, "CODE1", r"([%pLN_.]+)"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::named(1, "DISPLAY1", r"[%g]([^%g]+)[%g]"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::named(1, "DISPLAY2", r"[%g]([^%g]+)[%g]"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::named(1, "CODE2", r"([%pLN_.]+)"),
                ]),
                RegexTree::named(1, "CODE3", r"([%pLN_.]+)"),
                RegexTree::named(1, "CODE4", r"[%g]([^%g]+)[%g]"),
            ]),
            RegexTree::spaces_zero_or_more(),
            stereo::tags_pattern("TAGS1"),
            RegexTree::optional(RegexTree::named(
                1,
                "STEREOGROUP",
                r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)",
            )),
            stereo::tags_pattern("TAGS2"),
            RegexTree::spaces_zero_or_more(),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(
                2,
                "LINECOLOR",
                r"##(?:\[(dotted|dashed|bold)\])?(\w+)?",
            )),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r":"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "ADDFIELD", r"(.*)"),
            ])),
            RegexTree::end(),
        ]),
    )
    .in_passes(&[ParserPass::One, ParserPass::Two, ParserPass::Three])
    .boxed()
}

/// PlantUML's `CommandLinkState`.
pub(super) fn link_state<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandLinkState",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "ENT1", r"([%pLN_.:]+|[%pLN_.:]+\[H\*?\]|\[\*\]|\[H\*?\]|(?:==+)(?:[%pLN_.:]+)(?:==+))"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::concat(vec![
                RegexTree::named(1, "ARROW_CROSS_START", r"(x)?"),
                RegexTree::named(1, "ARROW_BODY1", r"(-+)"),
                RegexTree::named(1, "ARROW_STYLE1", r"(?:\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)\])?"),
                RegexTree::named(1, "ARROW_DIRECTION", r"(left|right|up|down|le?|ri?|up?|do?)?"),
                RegexTree::named(1, "ARROW_STYLE2", r"(?:\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)\])?"),
                RegexTree::named(1, "ARROW_BODY2", r"(-*)"),
                RegexTree::leaf(r"\>"),
                RegexTree::named(1, "ARROW_CIRCLE_END", r"(o[%s]+)?"),
            ]),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "ENT2", r"([%pLN_.:]+|[%pLN_.:]+\[H\*?\]|\[\*\]|\[H\*?\]|(?:==+)(?:[%pLN_.:]+)(?:==+))"),
            RegexTree::spaces_zero_or_more(),
            stereo::optional_pattern("STEREOTYPE"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r":"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "LABEL", r"(.+)"),
            ])),
            RegexTree::end(),
        ]),
    )
        .in_passes(&[ParserPass::Two])
    .boxed()
}

/// PlantUML's `CommandLinkStateReverse`.
pub(super) fn link_state_reverse<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandLinkStateReverse",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "ENT2", r"([%pLN_.:]+|[%pLN_.:]+\[H\*?\]|\[\*\]|\[H\*?\]|(?:==+)(?:[%pLN_.:]+)(?:==+))"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::concat(vec![
                RegexTree::named(1, "ARROW_CIRCLE_END", r"(o[%s]+)?"),
                RegexTree::leaf(r"\<"),
                RegexTree::named(1, "ARROW_BODY2", r"(-*)"),
                RegexTree::named(1, "ARROW_STYLE2", r"(?:\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)\])?"),
                RegexTree::named(1, "ARROW_DIRECTION", r"(left|right|up|down|le?|ri?|up?|do?)?"),
                RegexTree::named(1, "ARROW_STYLE1", r"(?:\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)\])?"),
                RegexTree::named(1, "ARROW_BODY1", r"(-+)"),
                RegexTree::named(1, "ARROW_CROSS_START", r"(x)?"),
            ]),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "ENT1", r"([%pLN_.:]+|[%pLN_.:]+\[H\*?\]|\[\*\]|\[H\*?\]|(?:==+)(?:[%pLN_.:]+)(?:==+))"),
            RegexTree::spaces_zero_or_more(),
            stereo::optional_pattern("STEREOTYPE"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r":"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "LABEL", r"(.+)"),
            ])),
            RegexTree::end(),
        ]),
    )
        .in_passes(&[ParserPass::Two])
    .boxed()
}

/// PlantUML's `CommandCreatePackageState`.
pub(super) fn create_package_state<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandCreatePackageState",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"state"),
            RegexTree::spaces_one_or_more(),
            RegexTree::or(vec![
                RegexTree::concat(vec![
                    RegexTree::named(1, "CODE1", r"([%pLN_.]+)"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::named(1, "DISPLAY1", r"[%g]([^%g]+)[%g]"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::optional(RegexTree::concat(vec![
                        RegexTree::named(1, "DISPLAY2", r"[%g]([^%g]+)[%g]"),
                        RegexTree::spaces_one_or_more(),
                        RegexTree::leaf(r"as"),
                        RegexTree::spaces_one_or_more(),
                    ])),
                    RegexTree::named(1, "CODE2", r"([%pLN_.]+)"),
                ]),
            ]),
            RegexTree::spaces_zero_or_more(),
            stereo::tags_pattern("TAGS1"),
            RegexTree::optional(RegexTree::named(
                1,
                "STEREOGROUP",
                r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)",
            )),
            stereo::tags_pattern("TAGS2"),
            RegexTree::spaces_zero_or_more(),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(
                2,
                "LINECOLOR",
                r"##(?:\[(dotted|dashed|bold)\])?(\w+)?",
            )),
            RegexTree::leaf(r"(?:[%s]*\{|[%s]+begin)"),
            RegexTree::end(),
        ]),
    )
    .in_passes(&[ParserPass::One, ParserPass::Two, ParserPass::Three])
    .boxed()
}

/// PlantUML's `CommandCreatePackage2`.
pub(super) fn create_package2<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandCreatePackage2",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"frame"),
            RegexTree::spaces_one_or_more(),
            RegexTree::or(vec![
                RegexTree::concat(vec![
                    RegexTree::named(1, "CODE1", r"([%pLN_.]+)"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::named(1, "DISPLAY1", r"[%g]([^%g]+)[%g]"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::optional(RegexTree::concat(vec![
                        RegexTree::named(1, "DISPLAY2", r"[%g]([^%g]+)[%g]"),
                        RegexTree::spaces_one_or_more(),
                        RegexTree::leaf(r"as"),
                        RegexTree::spaces_one_or_more(),
                    ])),
                    RegexTree::named(1, "CODE2", r"([%pLN_.]+)"),
                ]),
            ]),
            stereo::optional_pattern("STEREOTYPE"),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(
                2,
                "LINECOLOR",
                r"##(?:\[(dotted|dashed|bold)\])?(\w+)?",
            )),
            RegexTree::leaf(r"(?:[%s]*\{|[%s]+begin)"),
            RegexTree::end(),
        ]),
    )
    .in_passes(&[ParserPass::One, ParserPass::Two, ParserPass::Three])
    .boxed()
}

/// PlantUML's `CommandEndState`.
pub(super) fn end_state<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandEndState",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::counted(1, r"(end[%s]?state|\})"),
            RegexTree::end(),
        ]),
    )
    .in_passes(&[ParserPass::One, ParserPass::Two, ParserPass::Three])
    .boxed()
}

/// PlantUML's `CommandAddField`.
pub(super) fn add_field<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandAddField",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::or(vec![
                RegexTree::named(1, "CODE3", r"([%pLN_.]+)"),
                RegexTree::named(1, "CODE4", r"[%g]([^%g]+)[%g]"),
            ]),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r":"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "FIELD", r"(.*)"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandConcurrentState`.
pub(super) fn concurrent_state<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandConcurrentState",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "TYPE", r"(--+|\|\|+)"),
            RegexTree::end(),
        ]),
    )
    .in_passes(&[ParserPass::One, ParserPass::Two, ParserPass::Three])
    .boxed()
}
