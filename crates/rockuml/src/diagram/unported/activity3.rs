//! The commands of activity diagrams (`ActivityDiagramFactory3`).

use crate::command::Command;
use crate::command::unported::{self, NotPortedCommands};
use crate::klimt::url::Url;
use crate::pattern::{RegexTree, plantuml_regex};
use crate::{color, stereo};

/// PlantUML's `CommandSwimlane`.
pub(super) fn swimlane<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandSwimlane",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"\|"),
            RegexTree::named(1, "COLOR", r"(?:(#\w+[-\\|/]?\w+)\|)?"),
            RegexTree::named(1, "SWIMLANE", r"([^|]+)"),
            RegexTree::leaf(r"\|"),
            RegexTree::named(1, "LABEL", r"([^|]+)?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandSwimlane2`.
pub(super) fn swimlane2<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandSwimlane2",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"swimlane"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "COLOR", r"(?:(#\w+[-\\|/]?\w+))?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "SWIMLANE", r"([^|]+?)"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"as"),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(1, "LABEL", r"([^|]+)"),
            ])),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandPartition3`.
pub(super) fn partition3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandPartition3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "TYPE", r"(partition|package|rectangle|card|group)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                color::optional_pattern("BACK1"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(1, "NAME", r"([%g][^%g]+[%g]|.*?)"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                color::optional_pattern("BACK2"),
            ])),
            stereo::optional_pattern("STEREO"),
            RegexTree::named(1, "BRACKET", r"(\{?)"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandCloseGroup3`.
pub(super) fn close_group3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandCloseGroup3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"\}"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandCloseGroupLegacy3`.
pub(super) fn close_group_legacy3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandCloseGroupLegacy3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "CMD", r"(end ?group|group ?end)"),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandArrow3`.
pub(super) fn arrow3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandArrow3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::or(vec![
                RegexTree::leaf(r"->"),
                RegexTree::named(1, "COLOR", r"-\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*(?:(?:;(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)*)*)\]->"),
            ]),
            RegexTree::spaces_zero_or_more(),
            RegexTree::or(vec![
                RegexTree::named(1, "LABEL", r"(.*);"),
                RegexTree::leaf(r""),
            ]),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandArrowLong3`.
pub(super) fn arrow_long3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    Box::new(
        unported::multi_line(
            "CommandArrowLong3",
            RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::or(vec![
                RegexTree::leaf(r"->"),
                RegexTree::named(1, "COLOR", r"-\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*(?:(?:;(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)*)*)\]->"),
            ]),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "LABEL", r"(.*)"),
            RegexTree::end(),
        ]),
            &plantuml_regex(r"^(.*);$"),
        )
        .skipping_quote_lines(),
    )
}

/// PlantUML's `CommandRepeat3`.
pub(super) fn repeat3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandRepeat3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "COLOR", r"(?:(#\w+[-\\|/]?\w+):)?"),
            RegexTree::leaf(r"repeat"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r":"),
                RegexTree::named(1, "LABEL", r"(.*?)"),
                RegexTree::leaf(r";"),
                RegexTree::spaces_zero_or_more(),
            ])),
            RegexTree::optional(RegexTree::named(
                1,
                "STEREOGROUP",
                r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)",
            )),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandActivity3`.
pub(super) fn activity3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandActivity3",
        RegexTree::concat(vec![
            RegexTree::start(),
            stereo::optional_pattern("IGNORED"),
            RegexTree::leaf(r":"),
            RegexTree::named(1, "LABEL", r"(.*?)"),
            RegexTree::leaf(r";"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(
                1,
                "STEREOGROUP",
                r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)",
            )),
            RegexTree::spaces_zero_or_more(),
            Url::optional_pattern(),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandIf4`.
pub(super) fn if4<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandIf4",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "COLOR", r"(?:(#\w+[-\\|/]?\w+):)?"),
            RegexTree::leaf(r"if"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\("),
            RegexTree::named(1, "TEST", r"(.*?)"),
            RegexTree::leaf(r"\)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::counted(1, r"(is|equals?)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\("),
            RegexTree::named(1, "WHEN", r"(.+?)"),
            RegexTree::leaf(r"\)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"then"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r";?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(
                1,
                "STEREOGROUP",
                r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)",
            )),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandIf2`.
pub(super) fn if2<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandIf2",
        RegexTree::concat(vec![
            RegexTree::start(),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "COLOR", r"(?:(#\w+[-\\|/]?\w+):)?"),
            RegexTree::leaf(r"if"),
            stereo::optional_pattern("IGNORED"),
            RegexTree::leaf(r"\("),
            RegexTree::named(1, "TEST", r"(.*?)"),
            RegexTree::leaf(r"\)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r"then"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::optional(RegexTree::named(1, "WHEN", r"\((.+?)\)")),
            ])),
            RegexTree::leaf(r";?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(
                1,
                "STEREOGROUP",
                r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)",
            )),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandDecoratorMultine`.
pub(super) fn if2_multine<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandIf2",
        RegexTree::concat(vec![
            RegexTree::start(),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "COLOR", r"(?:(#\w+[-\\|/]?\w+):)?"),
            RegexTree::leaf(r"if"),
            stereo::optional_pattern("IGNORED"),
            RegexTree::leaf(r"\("),
            RegexTree::named(1, "TEST", r"(.*?)"),
            RegexTree::leaf(r"\)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r"then"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::optional(RegexTree::named(1, "WHEN", r"\((.+?)\)")),
            ])),
            RegexTree::leaf(r";?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(
                1,
                "STEREOGROUP",
                r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)",
            )),
            RegexTree::end(),
        ]),
    )
    .over_lines(50)
}

/// PlantUML's `CommandIfLegacy1`.
pub(super) fn if_legacy1<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandIfLegacy1",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"if"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\("),
            RegexTree::named(1, "TEST", r"(.+?)"),
            RegexTree::leaf(r"\)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"then"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"when"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "WHEN", r"(.*)"),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandElseIf3`.
pub(super) fn else_if3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandElseIf3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "COLOR", r"(?:(#\w+[-\\|/]?\w+):)?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r"\("),
                RegexTree::optional(RegexTree::or(vec![
                    RegexTree::leaf(r"->"),
                    RegexTree::named(1, "INCOMING_COLOR", r"-\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*(?:(?:;(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)*)*)\]->"),
                ])),
                RegexTree::named(1, "INCOMING", r"(.*?)"),
                RegexTree::leaf(r"\)"),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"else"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"if"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\("),
            RegexTree::named(1, "TEST", r"(.*?)"),
            RegexTree::leaf(r"\)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::counted(1, r"(is|equals?)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\("),
            RegexTree::named(1, "WHEN", r"(.+?)"),
            RegexTree::leaf(r"\)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r"then"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::optional(RegexTree::concat(vec![
                    RegexTree::leaf(r"\("),
                    RegexTree::optional(RegexTree::or(vec![
                        RegexTree::leaf(r"->"),
                        RegexTree::named(1, "WHEN_COLOR", r"-\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*(?:(?:;(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)*)*)\]->"),
                    ])),
                    RegexTree::leaf(r"\)"),
                ])),
            ])),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandElseIf2`.
pub(super) fn else_if2<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandElseIf2",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "COLOR", r"(?:(#\w+[-\\|/]?\w+):)?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r"\("),
                RegexTree::optional(RegexTree::or(vec![
                    RegexTree::leaf(r"->"),
                    RegexTree::named(1, "INCOMING_COLOR", r"-\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*(?:(?:;(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)*)*)\]->"),
                ])),
                RegexTree::named(1, "INCOMING", r"(.*?)"),
                RegexTree::leaf(r"\)"),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"else"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"if"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\("),
            RegexTree::named(1, "TEST", r"(.*?)"),
            RegexTree::leaf(r"\)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r"then"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::optional(RegexTree::concat(vec![
                    RegexTree::leaf(r"\("),
                    RegexTree::optional(RegexTree::or(vec![
                        RegexTree::leaf(r"->"),
                        RegexTree::named(1, "WHEN_COLOR", r"-\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*(?:(?:;(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)*)*)\]->"),
                    ])),
                    RegexTree::named(1, "WHEN", r"(.*?)"),
                    RegexTree::leaf(r"\)"),
                ])),
            ])),
            RegexTree::leaf(r";?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(1, "STEREOGROUP", r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)")),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandElse3`.
pub(super) fn else3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandElse3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"else"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r"\("),
                RegexTree::optional(RegexTree::or(vec![
                    RegexTree::leaf(r"->"),
                    RegexTree::named(1, "WHEN_COLOR", r"-\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*(?:(?:;(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)*)*)\]->"),
                ])),
                RegexTree::named(1, "WHEN", r"(.*?)"),
                RegexTree::leaf(r"\)"),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandDecoratorMultine`.
pub(super) fn else3_multine<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandElse3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"else"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r"\("),
                RegexTree::optional(RegexTree::or(vec![
                    RegexTree::leaf(r"->"),
                    RegexTree::named(1, "WHEN_COLOR", r"-\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*(?:(?:;(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)*)*)\]->"),
                ])),
                RegexTree::named(1, "WHEN", r"(.*?)"),
                RegexTree::leaf(r"\)"),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
    )
    .over_lines(50)
}

/// PlantUML's `CommandElseLegacy1`.
pub(super) fn else_legacy1<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandElseLegacy1",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"else"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"when"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "WHEN", r"(.*)"),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandEndif3`.
pub(super) fn endif3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandEndif3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"end"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"if"),
            RegexTree::leaf(r";?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(
                1,
                "STEREOGROUP",
                r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)",
            )),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandSwitch`.
pub(super) fn switch<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandSwitch",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "COLOR", r"(?:(#\w+[-\\|/]?\w+):)?"),
            RegexTree::leaf(r"switch"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\("),
            RegexTree::named(1, "TEST", r"(.*?)"),
            RegexTree::leaf(r"\)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(
                1,
                "STEREOGROUP",
                r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)",
            )),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandCase`.
pub(super) fn case<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandCase",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"case"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\("),
            RegexTree::named(1, "TEST", r"(.*?)"),
            RegexTree::leaf(r"\)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandEndSwitch`.
pub(super) fn end_switch<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandEndSwitch",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"endswitch"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(
                1,
                "STEREOGROUP",
                r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)",
            )),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandRepeatWhile3`.
pub(super) fn repeat_while3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandRepeatWhile3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"repeat"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"while"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::or(vec![
                RegexTree::concat(vec![
                    RegexTree::named(1, "TEST3", r"\((.*?)\)"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::counted(1, r"(is|equals?)"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::named(1, "WHEN3", r"\((.+?)\)"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::counted(1, r"(not)"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::named(1, "OUT3", r"\((.+?)\)"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::named(1, "TEST4", r"\((.*?)\)"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::counted(1, r"(not)"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::named(1, "OUT4", r"\((.+?)\)"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::named(1, "TEST2", r"\((.*?)\)"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::counted(1, r"(is|equals?)"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::named(1, "WHEN2", r"\((.+?)\)"),
                ]),
                RegexTree::optional(RegexTree::named(1, "TEST1", r"\((.*)\)")),
            ]),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::or(vec![
                    RegexTree::leaf(r"->"),
                    RegexTree::named(1, "XCOLOR", r"-\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*(?:(?:;(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)*)*)\]->"),
                ]),
                RegexTree::spaces_zero_or_more(),
                RegexTree::or(vec![
                    RegexTree::named(1, "LABEL", r"(.*)"),
                    RegexTree::leaf(r""),
                ]),
            ])),
            RegexTree::leaf(r";?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(1, "STEREOGROUP", r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)")),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandRepeatWhile3Multilines`.
pub(super) fn repeat_while3_multilines<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    Box::new(
        unported::multi_line3(
            "CommandRepeatWhile3Multilines",
            RegexTree::concat(vec![
                RegexTree::start(),
                RegexTree::leaf(r"repeat"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::leaf(r"while"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::leaf(r"\("),
                RegexTree::named(1, "TEST1", r"(.*)"),
                RegexTree::end(),
            ]),
            RegexTree::concat(vec![
                RegexTree::named(1, "TEST1", r"(.*)"),
                RegexTree::leaf(r"\)"),
                RegexTree::leaf(r";?"),
                RegexTree::end(),
            ]),
            true,
        )
        .skipping_quote_lines(),
    )
}

/// PlantUML's `CommandBackward3`.
pub(super) fn backward3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandBackward3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r"\("),
                RegexTree::optional(RegexTree::or(vec![
                    RegexTree::leaf(r"->"),
                    RegexTree::named(1, "INCOMING_COLOR", r"-\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*(?:(?:;(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)*)*)\]->"),
                ])),
                RegexTree::named(1, "INCOMING", r"(.*?)"),
                RegexTree::leaf(r"\)"),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"backward"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r":"),
            RegexTree::named(1, "LABEL", r"(.*?)"),
            RegexTree::leaf(r";"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(1, "STEREOGROUP", r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)")),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r"\("),
                RegexTree::optional(RegexTree::or(vec![
                    RegexTree::leaf(r"->"),
                    RegexTree::named(1, "OUTCOMING_COLOR", r"-\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*(?:(?:;(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)*)*)\]->"),
                ])),
                RegexTree::named(1, "OUTCOMING", r"(.*?)"),
                RegexTree::leaf(r"\)"),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandBackwardLong3`.
pub(super) fn backward_long3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    Box::new(
        unported::multi_line3(
            "CommandBackwardLong3",
            RegexTree::concat(vec![
                RegexTree::start(),
                RegexTree::spaces_zero_or_more(),
                RegexTree::leaf(r"backward"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::leaf(r":"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "DATA", r"(.*)"),
                RegexTree::end(),
            ]),
            RegexTree::concat(vec![
                RegexTree::named(1, "TEXT", r"(.*)"),
                RegexTree::leaf(r";"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::optional(RegexTree::named(
                    1,
                    "STEREOGROUP",
                    r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)",
                )),
                RegexTree::end(),
            ]),
            true,
        )
        .skipping_quote_lines(),
    )
}

/// PlantUML's `CommandWhile3`.
pub(super) fn while3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandWhile3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "COLOR", r"(?:(#\w+[-\\|/]?\w+):)?"),
            RegexTree::leaf(r"while"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\("),
            RegexTree::named(1, "TEST", r"(.*?)"),
            RegexTree::leaf(r"\)"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_zero_or_more(),
                RegexTree::counted(1, r"(is|equals?)"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "YES", r"\((.+?)\)"),
            ])),
            RegexTree::leaf(r";?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(
                1,
                "STEREOGROUP",
                r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)",
            )),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandWhileEnd3`.
pub(super) fn while_end3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandWhileEnd3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::or(vec![
                RegexTree::concat(vec![
                    RegexTree::leaf(r"end"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::leaf(r"while"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::leaf(r"while"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::leaf(r"end"),
                ]),
            ]),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(1, "OUT", r"\((.+?)\)")),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandFork3`.
pub(super) fn fork3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandFork3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"fork"),
            RegexTree::leaf(r";?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(
                1,
                "STEREOGROUP",
                r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)",
            )),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandForkAgain3`.
pub(super) fn fork_again3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandForkAgain3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"fork"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"again"),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandForkEnd3`.
pub(super) fn fork_end3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandForkEnd3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named_or(
                "STYLE",
                vec![
                    RegexTree::concat(vec![
                        RegexTree::leaf(r"end"),
                        RegexTree::spaces_zero_or_more(),
                        RegexTree::leaf(r"fork"),
                    ]),
                    RegexTree::concat(vec![
                        RegexTree::leaf(r"fork"),
                        RegexTree::spaces_zero_or_more(),
                        RegexTree::leaf(r"end"),
                    ]),
                    RegexTree::concat(vec![
                        RegexTree::leaf(r"end"),
                        RegexTree::spaces_zero_or_more(),
                        RegexTree::leaf(r"merge"),
                    ]),
                ],
            ),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "LABEL", r"(\{.+\})?"),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandSplit3`.
pub(super) fn split3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandSplit3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"split"),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandSplitAgain3`.
pub(super) fn split_again3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandSplitAgain3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"split"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"again"),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandSplitEnd3`.
pub(super) fn split_end3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandSplitEnd3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::or(vec![
                RegexTree::concat(vec![
                    RegexTree::leaf(r"end"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::leaf(r"split"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::leaf(r"split"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::leaf(r"end"),
                ]),
            ]),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandStart3`.
pub(super) fn start3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandStart3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"start"),
            RegexTree::leaf(r";?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(
                1,
                "STEREOGROUP",
                r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)",
            )),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandStop3`.
pub(super) fn stop3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandStop3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"stop"),
            RegexTree::leaf(r";?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(
                1,
                "STEREOGROUP",
                r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)",
            )),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandCircleSpot3`.
pub(super) fn circle_spot3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandCircleSpot3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "COLOR", r"(?:(#\w+[-\\|/]?\w+):)?"),
            RegexTree::named(1, "SPOT", r"\((\S)\)"),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandBreak`.
pub(super) fn break_command<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandBreak",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"break"),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandEnd3`.
pub(super) fn end3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandEnd3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"end"),
            RegexTree::leaf(r";?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(
                1,
                "STEREOGROUP",
                r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)",
            )),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandKill3`.
pub(super) fn kill3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandKill3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"kill|detach"),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandLink3`.
pub(super) fn link3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandLink3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"link"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "COLOR", r"(#\w+)"),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandNote3`.
pub(super) fn note3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandNote3",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "TYPE", r"(note|floating note)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "POSITION", r"(left|right)?"),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            stereo::tags_pattern("TAGS"),
            stereo::optional_pattern("STEREO"),
            RegexTree::leaf(r":"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "NOTE", r"(.*)"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandNoteLong3`.
pub(super) fn note_long3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    Box::new(
        unported::multi_line(
            "CommandNoteLong3",
            RegexTree::concat(vec![
                RegexTree::start(),
                RegexTree::named(1, "TYPE", r"(note|floating note)"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "POSITION", r"(left|right)?"),
                RegexTree::spaces_zero_or_more(),
                stereo::tags_pattern("TAGS"),
                stereo::optional_pattern("STEREO"),
                color::optional_pattern("COLOR"),
                RegexTree::end(),
            ]),
            &plantuml_regex(r"^end[%s]?note$"),
        )
        .skipping_quote_lines(),
    )
}

/// PlantUML's `CommandActivityLong3`.
pub(super) fn activity_long3<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    Box::new(
        unported::multi_line3(
            "CommandActivityLong3",
            RegexTree::concat(vec![
                RegexTree::start(),
                color::optional_pattern("COLOR"),
                RegexTree::leaf(r":"),
                RegexTree::named(1, "DATA", r"(.*)"),
                RegexTree::end(),
            ]),
            RegexTree::concat(vec![
                RegexTree::named(1, "TEXT", r"(.*)"),
                RegexTree::leaf(r";"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::optional(RegexTree::named(
                    1,
                    "STEREOGROUP",
                    r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)",
                )),
                RegexTree::spaces_zero_or_more(),
                Url::optional_pattern(),
                RegexTree::end(),
            ]),
            false,
        )
        .skipping_quote_lines(),
    )
}

/// PlantUML's `CommandActivityList`.
pub(super) fn activity_list<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandActivityList",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"[-*]"),
            RegexTree::optional(RegexTree::leaf(r"[%s]")),
            RegexTree::named(1, "LABEL", r"(.*?)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(
                1,
                "STEREOGROUP",
                r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)",
            )),
            RegexTree::spaces_zero_or_more(),
            Url::optional_pattern(),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandLabel`.
pub(super) fn label<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandLabel",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"label"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "NAME", r"([%pLN_.]+)"),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandGoto`.
pub(super) fn goto<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandGoto",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"goto"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "NAME", r"([%pLN_.]+)"),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandDecoratorMultine`.
pub(super) fn else_if2_multine<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandElseIf2",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "COLOR", r"(?:(#\w+[-\\|/]?\w+):)?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r"\("),
                RegexTree::optional(RegexTree::or(vec![
                    RegexTree::leaf(r"->"),
                    RegexTree::named(1, "INCOMING_COLOR", r"-\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*(?:(?:;(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)*)*)\]->"),
                ])),
                RegexTree::named(1, "INCOMING", r"(.*?)"),
                RegexTree::leaf(r"\)"),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"else"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"if"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\("),
            RegexTree::named(1, "TEST", r"(.*?)"),
            RegexTree::leaf(r"\)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r"then"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::optional(RegexTree::concat(vec![
                    RegexTree::leaf(r"\("),
                    RegexTree::optional(RegexTree::or(vec![
                        RegexTree::leaf(r"->"),
                        RegexTree::named(1, "WHEN_COLOR", r"-\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*(?:(?:;(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)*)*)\]->"),
                    ])),
                    RegexTree::named(1, "WHEN", r"(.*?)"),
                    RegexTree::leaf(r"\)"),
                ])),
            ])),
            RegexTree::leaf(r";?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(1, "STEREOGROUP", r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)")),
            RegexTree::end(),
        ]),
    )
    .over_lines(50)
}
