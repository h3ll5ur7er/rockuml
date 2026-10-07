//! Commands the diagrams of entities and links share (class, description and state diagrams), which
//! PlantUML keeps in its `command`, `classdiagram`, `descdiagram` and `objectdiagram` packages.

pub(super) mod note;

use super::titled::TitledDiagram;
use crate::command::Command;
use crate::command::unported::{self, NotPortedCommands};
use crate::command::{BlocLines, CommandControl, PatternCommand, SingleLine};
use crate::klimt::url::Url;
use crate::pattern::{RegexResult, RegexTree, plantuml_regex};
use crate::skin::Rankdir;
use crate::text::{LineLocation, StringLocated};
use crate::{color, stereo};

/// PlantUML's `CommandFootboxIgnored`.
pub(super) fn footbox_ignored<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandFootboxIgnored",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::counted(1, r"(hide|show)?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"footbox"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandRankDir`: `left to right direction` lays the diagram out sideways.
pub(super) fn rank_dir<D: TitledDiagram + 'static>() -> Box<dyn Command<D>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "DIRECTION", r"(left[%s]to[%s]right|top[%s]to[%s]bottom)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"direction"),
            RegexTree::end(),
        ]),
        |diagram: &mut D, _: &LineLocation, arg: &RegexResult| {
            let direction = arg.get("DIRECTION", 0).unwrap_or_default();
            let rankdir = if direction.to_ascii_lowercase().starts_with("left") {
                Rankdir::LeftToRight
            } else {
                Rankdir::TopToBottom
            };
            diagram.titled().skin.set_rankdir(rankdir);
            Ok(())
        },
    )))
}

/// PlantUML's `CommandNewpage`.
pub(super) fn newpage<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandNewpage",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"newpage"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandHideShow2`.
pub(super) fn hide_show2<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandHideShow2",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "COMMAND", r"(hide|hide-class|show|show-class)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "WHAT", r"([^%s]+|\<\<.*\>\>)"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandRemoveRestore`.
pub(super) fn remove_restore<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandRemoveRestore",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "COMMAND", r"(remove|restore)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "WHAT", r"(.+)"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandCreateMap`.
pub(super) fn create_map<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    Box::new(
        unported::multi_line(
            "CommandCreateMap",
            RegexTree::concat(vec![
                RegexTree::start(),
                RegexTree::named(0, "TYPE", r"map"),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(2, "NAME", r"(?:[%g]([^%g]+)[%g][%s]+as[%s]+)?([%pLN_.]+)"),
                stereo::optional_pattern("STEREO"),
                Url::optional_pattern(),
                RegexTree::spaces_zero_or_more(),
                color::optional_pattern("COLOR"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::optional(RegexTree::concat(vec![
                    RegexTree::leaf(r"##"),
                    RegexTree::named(2, "LINECOLOR", r"(?:\[(dotted|dashed|bold)\])?(\w+)?"),
                ])),
                RegexTree::spaces_zero_or_more(),
                RegexTree::leaf(r"\{"),
                RegexTree::end(),
            ]),
            &plantuml_regex(r"^[%s]*\}[%s]*$"),
        )
        .skipping_quote_lines(),
    )
}

/// PlantUML's `CommandCreateJson`.
pub(super) fn create_json<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    Box::new(
        unported::multi_line(
            "CommandCreateJson",
            RegexTree::concat(vec![
                RegexTree::start(),
                RegexTree::named(0, "TYPE", r"json"),
                RegexTree::spaces_one_or_more(),
                RegexTree::or(vec![
                    RegexTree::concat(vec![
                        RegexTree::named(1, "DISPLAY1", r"[%g]([^%g]*)[%g]"),
                        RegexTree::spaces_one_or_more(),
                        RegexTree::leaf(r"as"),
                        RegexTree::spaces_one_or_more(),
                        RegexTree::named(1, "CODE1", r"([^%s{}%g<>]+)"),
                    ]),
                    RegexTree::concat(vec![
                        RegexTree::named(1, "CODE2", r"([^%s{}%g<>]+)"),
                        RegexTree::spaces_one_or_more(),
                        RegexTree::leaf(r"as"),
                        RegexTree::spaces_one_or_more(),
                        RegexTree::named(1, "DISPLAY2", r"[%g]([^%g]*)[%g]"),
                    ]),
                    RegexTree::named(1, "CODE3", r"([^%s{}%g<>]+)"),
                    RegexTree::named(1, "CODE4", r"[%g]([^%g]+)[%g]"),
                ]),
                stereo::optional_pattern("STEREO"),
                Url::optional_pattern(),
                RegexTree::spaces_zero_or_more(),
                color::optional_pattern("COLOR"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::leaf(r"\{"),
                RegexTree::end(),
            ]),
            &plantuml_regex(r"^[%s]*\}[%s]*$"),
        )
        .skipping_quote_lines()
        .verified_by(json_is_complete),
    )
}

/// PlantUML's `CommandCreateJsonSingleLine`.
pub(super) fn create_json_single_line<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandCreateJsonSingleLine",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(0, "TYPE", r"json"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(2, "NAME", r"(?:[%g]([^%g]+)[%g][%s]+as[%s]+)?([%pLN_.]+)"),
            stereo::optional_pattern("STEREO"),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::or(vec![
                RegexTree::named(1, "DATA_BOOLEAN", r"(true|false)"),
                RegexTree::named(1, "DATA_NUMBER", r"(-?\d+)"),
                RegexTree::named(1, "DATA_NULL", r"(null)"),
                RegexTree::named(1, "DATA_STRING", r#"(".*")"#),
                RegexTree::named(1, "DATA_ARRAY", r"(\[.*\])"),
                RegexTree::named(
                    2,
                    "DATA_OBJECT",
                    r"(\{[%s]*[%g](\\[%g]|[^%g])+[%g][%s]*:.*\})",
                ),
            ]),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandEndPackage`.
pub(super) fn end_package<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandEndPackage",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"\}"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandPackageWithUSymbol`.
pub(super) fn package_with_usymbol<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandPackageWithUSymbol",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "SYMBOL", r"(package|rectangle|hexagon|node|artifact|folder|file|frame|cloud|action|process|database|storage|component|card|queue|stack)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::or(vec![
                RegexTree::concat(vec![
                    RegexTree::named(1, "DISPLAY1", r"([%g].+?[%g])"),
                    RegexTree::optional(RegexTree::concat(vec![
                        RegexTree::spaces_one_or_more(),
                        RegexTree::named(1, "STEREOTYPE1", r"(\<\<.+\>\>)"),
                    ])),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::named(1, "CODE1", r"([^#%s{}]+)"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::named(1, "CODE2", r"([^#%s{}%g]+)"),
                    RegexTree::optional(RegexTree::concat(vec![
                        RegexTree::spaces_one_or_more(),
                        RegexTree::named(1, "STEREOTYPE2", r"(\<\<.+\>\>)"),
                    ])),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::named(1, "DISPLAY2", r"([%g].+?[%g])"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::named(1, "DISPLAY3", r"([^#%s{}%g]+)"),
                    RegexTree::optional(RegexTree::concat(vec![
                        RegexTree::spaces_one_or_more(),
                        RegexTree::named(1, "STEREOTYPE3", r"(\<\<.+\>\>)"),
                    ])),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::named(1, "CODE3", r"([^#%s{}%g]+)"),
                ]),
                RegexTree::named(1, "CODE8", r"([%g][^%g]+[%g])"),
                RegexTree::named(1, "CODE9", r"([^#%s{}%g]*)"),
            ]),
            RegexTree::spaces_zero_or_more(),
            stereo::tags_pattern("TAGS1"),
            stereo::optional_pattern("STEREOTYPE"),
            stereo::tags_pattern("TAGS2"),
            RegexTree::spaces_zero_or_more(),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandTogether`.
pub(super) fn together<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandTogether",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"together"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandUrl`.
pub(super) fn url<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandUrl",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"url"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::leaf(r"of|for")),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "CODE", r"([%pLN_.]+|[%g][^%g]+[%g])"),
            RegexTree::spaces_one_or_more(),
            RegexTree::optional(RegexTree::leaf(r"is")),
            RegexTree::spaces_zero_or_more(),
            Url::mandatory_pattern(),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandCreateElementMultilines`.
pub(super) fn create_element_multilines_type0<D: NotPortedCommands + 'static>()
-> Box<dyn Command<D>> {
    Box::new(
        unported::multi_line(
            "CommandCreateElementMultilines",
            RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "TYPE", r"(person|artifact|actor/|actor|folder|card|file|package|rectangle|hexagon|label|node|frame|cloud|action|process|database|queue|stack|storage|agent|usecase/|usecase|component|boundary|control|entity|interface|circle|collections|port|portin|portout)[%s]+"),
            RegexTree::named(1, "CODE", r"([%pLN_.]+)"),
            stereo::optional_pattern("STEREO"),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"as"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"[%g]"),
            RegexTree::named(1, "DESC", r"([^%g]*)"),
            RegexTree::end(),
        ]),
            &plantuml_regex(r"^(.*)[%g]$"),
        )
        .skipping_quote_lines(),
    )
}

/// PlantUML's `CommandCreateElementMultilines`.
pub(super) fn create_element_multilines_type1<D: NotPortedCommands + 'static>()
-> Box<dyn Command<D>> {
    Box::new(
        unported::multi_line(
            "CommandCreateElementMultilines",
            RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "TYPE", r"(person|artifact|actor/|actor|folder|card|file|package|rectangle|hexagon|label|node|frame|cloud|action|process|database|queue|stack|storage|agent|usecase/|usecase|component|boundary|control|entity|interface|circle|collections|port|portin|portout)[%s]+"),
            RegexTree::named(1, "CODE", r"([%pLN_.]+)"),
            stereo::optional_pattern("STEREO"),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\["),
            RegexTree::named(1, "DESC", r"(.*)"),
            RegexTree::end(),
        ]),
            &plantuml_regex(r"^([^\[\]]*)\]$"),
        )
        .skipping_quote_lines(),
    )
}

/// Whether the block holds JSON data, with or without the braces around it
/// (`CommandCreateJson.finalVerification`); until it does, more lines may complete it.
fn json_is_complete(lines: &BlocLines) -> CommandControl {
    let inner: String = lines
        .sub_extract(1, 1)
        .iter()
        .map(StringLocated::text)
        .collect();
    if crate::json::parse(&format!("{{{inner}}}")).is_ok() || crate::json::parse(&inner).is_ok() {
        CommandControl::Ok
    } else {
        CommandControl::OkPartial
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_blocks_last_until_their_data_parses() {
        let complete = |lines: &[&str]| json_is_complete(&BlocLines::from_texts(lines));
        assert_eq!(
            complete(&["json J {", r#""a": 1"#, "}"]),
            CommandControl::Ok
        );
        assert_eq!(
            complete(&["json J {", r#""a": ["#, "}"]),
            CommandControl::OkPartial
        );
        assert_eq!(
            complete(&["json J {", r#""a": [1,"#, "2]", "}"]),
            CommandControl::Ok
        );
    }
}
