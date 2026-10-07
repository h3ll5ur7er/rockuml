//! The commands of timing diagrams (`TimingDiagramFactory`).

use crate::command::Command;
use crate::command::unported::{self, NotPortedCommands};
use crate::pattern::{RegexTree, plantuml_regex};
use crate::{color, stereo};

/// PlantUML's `CommandRobustConcise`.
pub(super) fn robust_concise<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandRobustConcise",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "COMPACT", r"(compact)"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(1, "TYPE", r"(robust|concise|rectangle)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "FULL", r"[%g]([^%g]+)[%g]"),
                stereo::optional_pattern("STEREOTYPE"),
                RegexTree::leaf(r"as"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(1, "CODE", r"([%pLN_.@]+)"),
            stereo::optional_pattern("STEREOTYPE2"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::or(vec![color::optional_pattern("COLOR")]),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandClock`.
pub(super) fn clock<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandClock",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "COMPACT", r"(compact)"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(0, "TYPE", r"clock"),
            RegexTree::spaces_one_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "FULL", r"[%g]([^%g]+)[%g]"),
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"as"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(1, "CODE", r"([%pLN_.@]+)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"with"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"period"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "PERIOD", r"([0-9]+(?:\.[0-9]+)?)"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"pulse"),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(1, "PULSE", r"([0-9]+(?:\.[0-9]+)?)"),
            ])),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"offset"),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(1, "OFFSET", r"([0-9]+(?:\.[0-9]+)?)"),
            ])),
            RegexTree::spaces_zero_or_more(),
            stereo::optional_pattern("STEREO"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandAnalog`.
pub(super) fn analog<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandAnalog",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "COMPACT", r"(compact)"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::leaf(r"analog"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "FULL", r"[%g]([^%g]+)[%g]"),
            stereo::optional_pattern("STEREOTYPE"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::or(vec![RegexTree::leaf(r"between"), RegexTree::leaf(r"from")]),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(1, "START", r"(-?[0-9]*\.?[0-9]+)"),
                RegexTree::spaces_one_or_more(),
                RegexTree::or(vec![RegexTree::leaf(r"and"), RegexTree::leaf(r"to")]),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(1, "END", r"(-?[0-9]*\.?[0-9]+)"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::leaf(r"as"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "CODE", r"([%pLN_.@]+)"),
            stereo::optional_pattern("STEREOTYPE2"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandBinary`.
pub(super) fn binary<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandBinary",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "COMPACT", r"(compact)"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::leaf(r"binary"),
            RegexTree::spaces_one_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "FULL", r"[%g]([^%g]+)[%g]"),
                stereo::optional_pattern("STEREOTYPE"),
                RegexTree::leaf(r"as"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(1, "CODE", r"([%pLN_.@]+)"),
            stereo::optional_pattern("STEREOTYPE2"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandDefineStateShort`.
pub(super) fn define_state_short<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandDefineStateShort",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "PLAYER", r"([%pLN_.@]+)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"has"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "STATE", r"([-%pLN_.@]+)"),
            RegexTree::named(3, "STATES", r"((,([-%pLN_.@]+))*)"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandDefineStateLong`.
pub(super) fn define_state_long<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandDefineStateLong",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "PLAYER", r"([%pLN_.@]+)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"has"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "LABEL", r"[%g]([^%g]+)[%g]"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"as"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "STATE", r"([%pLN_.@]+)"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandChangeStateByPlayerCode`.
pub(super) fn change_state_by_player_code<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandChangeStateByPlayerCode",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "CODE", r"([\p{L}_][%pLN_.]*)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"is"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::or(vec![
                RegexTree::named(1, "STATE1", r"[%g]([^%g]*)[%g]"),
                RegexTree::named(1, "STATE2", r"([-%pLN_][-%pLN_.]*)"),
                RegexTree::named(1, "STATE3", r"(\{hidden\})"),
                RegexTree::named(1, "STATE4", r"(\{\.\.\.\})"),
                RegexTree::named(1, "STATE5", r"(\{-\})"),
                RegexTree::named(1, "STATE6", r"(\{\?\})"),
                RegexTree::named(
                    2,
                    "STATE7",
                    r"(?:\{([-%pLN_][-%pLN_.]*),([-%pLN_][-%pLN_.]*)\})",
                ),
            ]),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_zero_or_more(),
                RegexTree::leaf(r":"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "COMMENT", r"(.*?)"),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandChangeStateByTime`.
pub(super) fn change_state_by_time<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandChangeStateByTime",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::spaces_zero_or_more(),
            RegexTree::or(vec![
                RegexTree::named(2, "TIMECODE", r":([%pLN_.]+)([-+][.\d]+)?"),
                RegexTree::named(3, "TIMEDATE", r"(\d+)/(\d+)/(\d+)"),
                RegexTree::named(3, "TIMEHOUR", r"(\d+):(\d+):(\d+)"),
                RegexTree::named(2, "TIMEDIGIT", r"(\+?)(-?\d+\.?\d*)"),
                RegexTree::named(2, "TIMECLOCK", r"([%pLN_.@]+)\*(\d+)"),
            ]),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"is"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::or(vec![
                RegexTree::named(1, "STATE1", r"[%g]([^%g]*)[%g]"),
                RegexTree::named(1, "STATE2", r"([-%pLN_][-%pLN_.]*)"),
                RegexTree::named(1, "STATE3", r"(\{hidden\})"),
                RegexTree::named(1, "STATE4", r"(\{\.\.\.\})"),
                RegexTree::named(1, "STATE5", r"(\{-\})"),
                RegexTree::named(1, "STATE6", r"(\{\?\})"),
                RegexTree::named(
                    2,
                    "STATE7",
                    r"(?:\{([-%pLN_][-%pLN_.]*),([-%pLN_][-%pLN_.]*)\})",
                ),
            ]),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_zero_or_more(),
                RegexTree::leaf(r":"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "COMMENT", r"(.*?)"),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandAtTime`.
pub(super) fn at_time<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandAtTime",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::concat(vec![
                RegexTree::leaf(r"@"),
                RegexTree::or(vec![
                    RegexTree::named(2, "TIMECODE", r":([%pLN_.]+)([-+][.\d]+)?"),
                    RegexTree::named(3, "TIMEDATE", r"(\d+)/(\d+)/(\d+)"),
                    RegexTree::named(3, "TIMEHOUR", r"(\d+):(\d+):(\d+)"),
                    RegexTree::named(2, "TIMEDIGIT", r"(\+?)(-?\d+\.?\d*)"),
                    RegexTree::named(2, "TIMECLOCK", r"([%pLN_.@]+)\*(\d+)"),
                ]),
            ]),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"as"),
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r":"),
                RegexTree::named(1, "CODE", r"([%pLN_.]+)"),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandAtPlayer`.
pub(super) fn at_player<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandAtPlayer",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"@"),
            RegexTree::named(1, "PLAYER", r"([\p{L}_][%pLN_.]*)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandTimeMessage`.
pub(super) fn time_message<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandTimeMessage",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "PART1", r"([\p{L}_][%pLN_.]*)"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r"@"),
                RegexTree::or(vec![
                    RegexTree::named(2, "TIME1CODE", r":([%pLN_.]+)([-+][.\d]+)?"),
                    RegexTree::named(3, "TIME1DATE", r"(\d+)/(\d+)/(\d+)"),
                    RegexTree::named(3, "TIME1HOUR", r"(\d+):(\d+):(\d+)"),
                    RegexTree::named(2, "TIME1DIGIT", r"(\+?)(-?\d+\.?\d*)"),
                    RegexTree::named(2, "TIME1CLOCK", r"([%pLN_.@]+)\*(\d+)"),
                ]),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "ARROW_BODY", r"(-+)"),
            RegexTree::named(1, "ARROW_STYLE", r"(?:\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)\])?"),
            RegexTree::named(0, "ARROW_HEAD", r"\>"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "PART2", r"([\p{L}_][%pLN_.]*)"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r"@"),
                RegexTree::or(vec![
                    RegexTree::named(2, "TIME2CODE", r":([%pLN_.]+)([-+][.\d]+)?"),
                    RegexTree::named(3, "TIME2DATE", r"(\d+)/(\d+)/(\d+)"),
                    RegexTree::named(3, "TIME2HOUR", r"(\d+):(\d+):(\d+)"),
                    RegexTree::named(2, "TIME2DIGIT", r"(\+?)(-?\d+\.?\d*)"),
                    RegexTree::named(2, "TIME2CLOCK", r"([%pLN_.@]+)\*(\d+)"),
                ]),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r":"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "MESSAGE", r"(.*)"),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandNote`.
pub(super) fn note<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandNote",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"note"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "POSITION", r"(top|bottom)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"of"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "CODE", r"([\p{L}_][%pLN_.]*)"),
            RegexTree::spaces_zero_or_more(),
            stereo::tags_pattern("TAGS"),
            stereo::optional_pattern("STEREO"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r":"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "NOTE", r"(.+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandNoteLong`.
pub(super) fn note_long<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    Box::new(
        unported::multi_line(
            "CommandNoteLong",
            RegexTree::concat(vec![
                RegexTree::start(),
                RegexTree::spaces_zero_or_more(),
                RegexTree::leaf(r"note"),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(1, "POSITION", r"(top|bottom)"),
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"of"),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(1, "CODE", r"([\p{L}_][%pLN_.]*)"),
                RegexTree::spaces_zero_or_more(),
                stereo::optional_pattern("STEREO"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::end(),
            ]),
            &plantuml_regex(r"^end[%s]?note$"),
        )
        .skipping_quote_lines(),
    )
}

/// PlantUML's `CommandConstraint`.
pub(super) fn constraint<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandConstraint",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(2, "PART1", r"(([\p{L}_][%pLN_.]*))?"),
            RegexTree::concat(vec![
                RegexTree::leaf(r"@"),
                RegexTree::or(vec![
                    RegexTree::named(2, "TIME1CODE", r":([%pLN_.]+)([-+][.\d]+)?"),
                    RegexTree::named(3, "TIME1DATE", r"(\d+)/(\d+)/(\d+)"),
                    RegexTree::named(3, "TIME1HOUR", r"(\d+):(\d+):(\d+)"),
                    RegexTree::named(2, "TIME1DIGIT", r"(\+?)(-?\d+\.?\d*)"),
                    RegexTree::named(2, "TIME1CLOCK", r"([%pLN_.@]+)\*(\d+)"),
                ]),
            ]),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\<"),
            RegexTree::counted(1, r"(-+)"),
            RegexTree::named(1, "ARROW_STYLE1", r"(?:\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)\])?"),
            RegexTree::counted(1, r"(-*)"),
            RegexTree::leaf(r"\>"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::concat(vec![
                RegexTree::leaf(r"@"),
                RegexTree::or(vec![
                    RegexTree::named(2, "TIME2CODE", r":([%pLN_.]+)([-+][.\d]+)?"),
                    RegexTree::named(3, "TIME2DATE", r"(\d+)/(\d+)/(\d+)"),
                    RegexTree::named(3, "TIME2HOUR", r"(\d+):(\d+):(\d+)"),
                    RegexTree::named(2, "TIME2DIGIT", r"(\+?)(-?\d+\.?\d*)"),
                    RegexTree::named(2, "TIME2CLOCK", r"([%pLN_.@]+)\*(\d+)"),
                ]),
            ]),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r":"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "MESSAGE", r"(.*)"),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandScalePixel`.
pub(super) fn scale_pixel<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandScalePixel",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"scale"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "TICK", r"(\d+)"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::optional(RegexTree::leaf(r"[%s]")),
                RegexTree::named(1, "UNIT", r"([smhdDyY])"),
            ])),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"as"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "PIXEL", r"(\d+)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"pixels?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandHideTimeAxis`.
pub(super) fn hide_time_axis<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandHideTimeAxis",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "COMMAND", r"(hide|manual)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"time"),
            RegexTree::leaf(r".?"),
            RegexTree::leaf(r"axis"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandHighlight`.
pub(super) fn highlight<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandHighlight",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"highlight"),
            RegexTree::spaces_one_or_more(),
            RegexTree::or(vec![
                RegexTree::named(2, "FROMCODE", r":([%pLN_.]+)([-+][.\d]+)?"),
                RegexTree::named(3, "FROMDATE", r"(\d+)/(\d+)/(\d+)"),
                RegexTree::named(3, "FROMHOUR", r"(\d+):(\d+):(\d+)"),
                RegexTree::named(2, "FROMDIGIT", r"(\+?)(-?\d+\.?\d*)"),
                RegexTree::named(2, "FROMCLOCK", r"([%pLN_.@]+)\*(\d+)"),
            ]),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"to"),
            RegexTree::spaces_one_or_more(),
            RegexTree::or(vec![
                RegexTree::named(2, "TOCODE", r":([%pLN_.]+)([-+][.\d]+)?"),
                RegexTree::named(3, "TODATE", r"(\d+)/(\d+)/(\d+)"),
                RegexTree::named(3, "TOHOUR", r"(\d+):(\d+):(\d+)"),
                RegexTree::named(2, "TODIGIT", r"(\+?)(-?\d+\.?\d*)"),
                RegexTree::named(2, "TOCLOCK", r"([%pLN_.@]+)\*(\d+)"),
            ]),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r":"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "CAPTION", r"(.*)"),
            ])),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandModeCompact`.
pub(super) fn mode_compact<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandModeCompact",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"mode"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"compact"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandTicks`.
pub(super) fn ticks<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandTicks",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "PLAYER", r"([%pLN_.@]+)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"ticks"),
            RegexTree::spaces_one_or_more(),
            RegexTree::or(vec![
                RegexTree::leaf(r"every"),
                RegexTree::concat(vec![
                    RegexTree::leaf(r"num"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::leaf(r"on"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::leaf(r"multiple"),
                ]),
            ]),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "NUM", r"([0-9]+)"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandPixelHeight`.
pub(super) fn pixel_height<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandPixelHeight",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "PLAYER", r"([%pLN_.@]+)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"is"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "NUM", r"([0-9]+)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"pixels?"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"height"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandUseDateFormat`.
pub(super) fn use_date_format<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandUseDateFormat",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"use"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"date"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"format"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "FORMAT", r"[%g]([^%g]+)[%g]"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}
