//! The regular-expression commands of legacy activity diagrams (`ActivityDiagramFactory`).

use crate::command::Command;
use crate::command::unported::{self, NotPortedCommands};
use crate::klimt::url::Url;
use crate::pattern::{RegexTree, plantuml_regex};
use crate::{color, stereo};

/// PlantUML's `CommandLinkLongActivity`.
pub(super) fn link_long_activity<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    Box::new(
        unported::multi_line(
            "CommandLinkLongActivity",
            RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::optional(RegexTree::named_or(
                "FIRST",
                vec![
                    RegexTree::named(2, "STAR", r"(\(\*(top)?\))"),
                    RegexTree::named(1, "CODE", r"([%pLN][%pLN_.]*)"),
                    RegexTree::named(1, "BAR", r"(?:==+)[%s]*([%pLN_.]+)[%s]*(?:==+)"),
                    RegexTree::named(2, "QUOTED", r"[%g]([^%g]+)[%g](?:[%s]+as[%s]+([%pLN_.]+))?"),
                ],
            )),
            stereo::optional_pattern("STEREOTYPE"),
            RegexTree::named(1, "BACKCOLOR", r"(#\w+)?"),
            RegexTree::spaces_zero_or_more(),
            Url::optional_pattern(),
            RegexTree::named(1, "ARROW_BODY1", r"([-.]+)"),
            RegexTree::named(1, "ARROW_STYLE1", r"(?:\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)\])?"),
            RegexTree::named(1, "ARROW_DIRECTION", r"(\*|left|right|up|down|le?|ri?|up?|do?)?"),
            RegexTree::named(1, "ARROW_STYLE2", r"(?:\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)\])?"),
            RegexTree::named(1, "ARROW_BODY2", r"([-.]*)"),
            RegexTree::leaf(r"\>"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(1, "BRACKET", r"\[([^\]*]+[^\]]*)\]")),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"[%g]"),
            RegexTree::named(1, "DESC", r"([^%g]*?)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
            &plantuml_regex(r"^[%s]*([^%g]*)[%g](?:[%s]+as[%s]+([%pLN][%pLN_.]*))?[%s]*(\<\<.*\>\>)?[%s]*(?:in[%s]+([%g][^%g]+[%g]|\S+))?[%s]*(#\w+)?$"),
        )
        .skipping_quote_lines(),
    )
}

/// PlantUML's `CommandFactoryNoteActivity$2`.
pub(super) fn note_activity<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandFactoryNoteActivity$2",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"note"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "POSITION", r"(right|left|top|bottom)"),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r":"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "NOTE", r"(.*)"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandFactoryNoteActivity$1`.
pub(super) fn note_activity_multi_line<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    Box::new(unported::multi_line(
        "CommandFactoryNoteActivity$1",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"note"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "POSITION", r"(right|left|top|bottom)"),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
        &plantuml_regex(r"^[%s]*end[%s]?note$"),
    ))
}

/// PlantUML's `CommandLinkActivity`.
pub(super) fn link_activity<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandLinkActivity",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::optional(RegexTree::named_or(
                "FIRST",
                vec![
                    RegexTree::named(2, "STAR", r"(\(\*(top)?\))"),
                    RegexTree::named(1, "CODE", r"([%pLN][%pLN_.]*)"),
                    RegexTree::named(1, "BAR", r"(?:==+)[%s]*([%pLN_.]+)[%s]*(?:==+)"),
                    RegexTree::named(2, "QUOTED", r"[%g]([^%g]+)[%g](?:[%s]+as[%s]+([%pLN_.]+))?"),
                ],
            )),
            stereo::optional_pattern("STEREOTYPE"),
            RegexTree::named(1, "BACKCOLOR", r"(#\w+[-\\|/]?\w+)?"),
            RegexTree::spaces_zero_or_more(),
            Url::optional_pattern(),
            RegexTree::named(1, "ARROW_BODY1", r"([-.]+)"),
            RegexTree::named(1, "ARROW_STYLE1", r"(?:\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)\])?"),
            RegexTree::named(1, "ARROW_DIRECTION", r"(\*|left|right|up|down|le?|ri?|up?|do?)?"),
            RegexTree::named(1, "ARROW_STYLE2", r"(?:\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)\])?"),
            RegexTree::named(1, "ARROW_BODY2", r"([-.]*)"),
            RegexTree::leaf(r"\>"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(1, "BRACKET", r"\[([^\]*]+[^\]]*)\]")),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named_or(
                "FIRST2",
                vec![
                    RegexTree::named(2, "STAR2", r"(\(\*(top|\d+)?\))"),
                    RegexTree::named(1, "OPENBRACKET2", r"(\{)"),
                    RegexTree::named(1, "CODE2", r"([%pLN][%pLN_.]*)"),
                    RegexTree::named(1, "BAR2", r"(?:==+)[%s]*([%pLN_.]+)[%s]*(?:==+)"),
                    RegexTree::named(2, "QUOTED2", r"[%g]([^%g]+)[%g](?:[%s]+as[%s]+([%pLN][%pLN_.]*))?"),
                    RegexTree::named(1, "QUOTED_INVISIBLE2", r"(\w.*?)"),
                ],
            ),
            stereo::optional_pattern("STEREOTYPE2"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r"in"),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(1, "PARTITION2", r"([%g][^%g]+[%g]|\S+)"),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "BACKCOLOR2", r"(#\w+[-\\|/]?\w+)?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}
