//! The commands of class and object diagrams. Most are not ported yet and only recognise their lines;
//! those whose failures decide whether the lines are a class diagram at all are.

use std::sync::LazyLock;

use regex::Regex;

use super::ClassDiagram;
use crate::command::unported::{self, NotPortedCommands};
use crate::command::{
    Command, CommandError, CommandResult, PatternCommand, SingleLine, SingleLineCommand,
};
use crate::klimt::url::Url;
use crate::pattern::{RegexResult, RegexTree, java_regex, plantuml_regex};
use crate::text::LineLocation;
use crate::{color, stereo};

/// PlantUML's `CommandAddMethod`.
pub(super) fn add_method<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandAddMethod",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "NAME", r"([%pLN_.]+|[%g][^%g]+[%g])"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r":"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "DATA", r"(.*)"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandCreateClassMultilines`.
pub(super) fn create_class_multilines<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    Box::new(
        unported::multi_line(
            "CommandCreateClassMultilines",
            RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "VISIBILITY", r"([-#+~])?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "TYPE", r"(interface|enum|annotation|abstract[%s]+class|static[%s]+class|abstract|class|entity|protocol|struct|exception|metaclass|stereotype|dataclass|record)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::or(vec![
                RegexTree::concat(vec![
                    RegexTree::named(2, "DISPLAY1", r"[%g](.+?)(?:\<([^\<\>/](?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<\>)*\>)*\>)*\>)*\>)*)\>)?[%g]"),
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
                    RegexTree::named(2, "DISPLAY2", r"[%g](.+?)(?:\<([^\<\>/](?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<\>)*\>)*\>)*\>)*\>)*)\>)?[%g]"),
                ]),
                RegexTree::named(1, "CODE3", r"([^%s{}%g<>]+)"),
                RegexTree::named(1, "CODE4", r"[%g]([^%g]+)[%g]"),
            ]),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "GENERIC", r"\<([^\<\>/](?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<\>)*\>)*\>)*\>)*\>)*)\>"),
            ])),
            RegexTree::spaces_zero_or_more(),
            stereo::tags_pattern("TAGS1"),
            stereo::optional_pattern("STEREO"),
            stereo::tags_pattern("TAGS2"),
            RegexTree::spaces_zero_or_more(),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r"##"),
                RegexTree::named(2, "LINECOLOR", r"(?:\[(dotted|dashed|bold)\])?(\w+)?"),
            ])),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::named(3, "EXTENDS", r"(extends)[%s]+((?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_$]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_$]+)*(?:\s*,\s*(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_$]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_$]+)*)*|[%g]([^%g]+)[%g])"),
                RegexTree::optional(RegexTree::concat(vec![
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::counted(1, r"\<([^\<\>/](?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<\>)*\>)*\>)*\>)*\>)*)\>"),
                ])),
            ])),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::named(3, "IMPLEMENTS", r"(implements)[%s]+((?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_$]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_$]+)*(?:\s*,\s*(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_$]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_$]+)*)*|[%g]([^%g]+)[%g])"),
                RegexTree::optional(RegexTree::concat(vec![
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::counted(1, r"\<([^\<\>/](?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<\>)*\>)*\>)*\>)*\>)*)\>"),
                ])),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
            &plantuml_regex(r"^[%s]*\}[%s]*$"),
        )
        .skipping_quote_lines()
        .with_final_bracket(),
    )
}

/// PlantUML's `CommandCreateEntityObjectMultilines`.
pub(super) fn create_entity_object_multilines<D: NotPortedCommands + 'static>()
-> Box<dyn Command<D>> {
    Box::new(
        unported::multi_line(
            "CommandCreateEntityObjectMultilines",
            RegexTree::concat(vec![
                RegexTree::start(),
                RegexTree::named(0, "TYPE", r"object"),
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
        .skipping_quote_lines(),
    )
}

/// PlantUML's `CommandCreateClass`.
pub(super) fn create_class<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandCreateClass",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "VISIBILITY", r"([-#+~])?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "TYPE", r"(interface|enum|annotation|abstract[%s]+class|static[%s]+class|abstract|class|entity|circle|diamond|protocol|struct|exception|metaclass|stereotype|dataclass|record|map)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::or(vec![
                RegexTree::concat(vec![
                    RegexTree::named(2, "DISPLAY1", r"[%g](.+?)(?:\<([^\<\>/](?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<\>)*\>)*\>)*\>)*\>)*)\>)?[%g]"),
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
                    RegexTree::named(2, "DISPLAY2", r"[%g](.+?)(?:\<([^\<\>/](?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<\>)*\>)*\>)*\>)*\>)*)\>)?[%g]"),
                ]),
                RegexTree::named(1, "CODE3", r"([^%s{}%g<>]+)"),
                RegexTree::named(1, "CODE4", r"[%g]([^%g]+)[%g]"),
            ]),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "GENERIC", r"\<([^\<\>/](?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<\>)*\>)*\>)*\>)*\>)*)\>"),
            ])),
            RegexTree::spaces_zero_or_more(),
            stereo::tags_pattern("TAGS1"),
            stereo::optional_pattern("STEREO"),
            stereo::tags_pattern("TAGS2"),
            RegexTree::spaces_zero_or_more(),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r"##"),
                RegexTree::named(2, "LINECOLOR", r"(?:\[(dotted|dashed|bold)\])?(\w+)?"),
            ])),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::named(3, "EXTENDS", r"(extends)[%s]+((?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_$]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_$]+)*(?:\s*,\s*(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_$]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_$]+)*)*|[%g]([^%g]+)[%g])"),
            ])),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::named(3, "IMPLEMENTS", r"(implements)[%s]+((?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_$]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_$]+)*(?:\s*,\s*(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_$]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_$]+)*)*|[%g]([^%g]+)[%g])"),
            ])),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_zero_or_more(),
                RegexTree::leaf(r"\{"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::leaf(r"\}"),
            ])),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandCreateEntityObject`.
pub(super) fn create_entity_object<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandCreateEntityObject",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(0, "TYPE", r"object"),
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
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandAllowMixing`: classes and description elements may mix.
pub(super) fn allow_mixing() -> Box<dyn Command<ClassDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf("allow"),
            RegexTree::leaf("_?"),
            RegexTree::leaf("mixing"),
            RegexTree::end(),
        ]),
        |diagram: &mut ClassDiagram, _: &LineLocation, _: &RegexResult| {
            diagram.allow_mixing = true;
            Ok(())
        },
    )))
}

/// PlantUML's `CommandCreateElementParenthesis`.
pub(super) fn create_element_parenthesis<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandCreateElementParenthesis",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"\(\)[%s]+"),
            color::optional_pattern("COLOR2"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::or(vec![
                RegexTree::named(1, "CODE1", r"([%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\]|[%g].+?[%g])"),
                RegexTree::concat(vec![
                    RegexTree::named(1, "DISPLAY2", r"([%g].+?[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
                    RegexTree::optional(RegexTree::concat(vec![
                        RegexTree::spaces_one_or_more(),
                        RegexTree::named(1, "STEREOTYPE2", r"(\<\<.+\>\>)"),
                    ])),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::named(1, "CODE2", r"([%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::named(1, "CODE3", r"([%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
                    RegexTree::optional(RegexTree::concat(vec![
                        RegexTree::spaces_one_or_more(),
                        RegexTree::named(1, "STEREOTYPE3", r"(\<\<.+\>\>)"),
                    ])),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::named(1, "DISPLAY3", r"([%g].+?[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::named(1, "DISPLAY4", r"([%g].+?[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\]|[%pLN_.]+)"),
                    RegexTree::optional(RegexTree::concat(vec![
                        RegexTree::spaces_one_or_more(),
                        RegexTree::named(1, "STEREOTYPE4", r"(\<\<.+\>\>)"),
                    ])),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::named(1, "CODE4", r"([%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
                ]),
            ]),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "STEREOTYPE", r"(\<\<.+\>\>)"),
            ])),
            RegexTree::spaces_zero_or_more(),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::end(),
        ]),
    )
        .forbidding(r"[\p{L}0-9_.]+")
    .boxed()
}

/// PlantUML's `CommandLayoutNewLine`.
pub(super) fn layout_new_line<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandLayoutNewLine",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"layout_new_line"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandPackage`.
pub(super) fn package<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandPackage",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "VISIBILITY", r"([-#+~])?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "TYPE", r"(package)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "NAME", r"([%g][^%g]+[%g]|[^#%s{}]*)"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"as"),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(1, "AS", r"([%pLN_.]+)"),
            ])),
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
    .with_final_bracket()
    .boxed()
}

/// PlantUML's `CommandPackageEmpty`.
pub(super) fn package_empty<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandPackageEmpty",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"package"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "DISPLAY", r"([%g][^%g]+[%g]|[^#%s{}]*)"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"as"),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(1, "CODE", r"([%pLN_.]+)"),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "COLOR", r"(#[0-9a-fA-F]{6}|#?\w+)?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\}"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// Whether description elements need the `mix_` prefix (`CommandCreateElementFull2.Mode`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    NormalKeyword,
    WithMixPrefix,
}

/// PlantUML's `CommandCreateElementFull2`: description elements in a class diagram. Without `allowmixing`,
/// only their `mix_` form is allowed.
pub(super) fn create_element_full2(mode: Mode) -> Box<dyn Command<ClassDiagram>> {
    let mut pattern = vec![RegexTree::start()];
    if mode == Mode::WithMixPrefix {
        pattern.push(RegexTree::leaf("mix_"));
    }
    pattern.extend([
        RegexTree::named(1, "SYMBOL", r"(state|person|artifact|actor/|actor|folder|card|file|package|rectangle|hexagon|label|node|frame|cloud|action|process|database|queue|stack|storage|agent|usecase/|usecase|component|boundary|control|entity|interface|circle|collections|port|portin|portout)"),
        RegexTree::spaces_one_or_more(),
        RegexTree::or(vec![
            RegexTree::named(1, "CODE1", r"([%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\]|[%g].+?[%g])"),
            RegexTree::concat(vec![
                RegexTree::named(1, "DISPLAY2", r"([%g].+?[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
                stereo::optional_pattern("STEREOTYPE2"),
                RegexTree::leaf("as"),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(1, "CODE2", r"([%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
            ]),
        ]),
        RegexTree::spaces_zero_or_more(),
        stereo::tags_pattern("TAGS1"),
        stereo::optional_pattern("STEREOTYPE"),
        stereo::tags_pattern("TAGS2"),
        RegexTree::spaces_zero_or_more(),
        Url::optional_pattern(),
        RegexTree::spaces_zero_or_more(),
        color::optional_pattern("COLOR"),
        RegexTree::end(),
    ]);
    Box::new(SingleLine(CreateElementFull2 {
        mode,
        pattern: RegexTree::concat(pattern),
    }))
}

struct CreateElementFull2 {
    mode: Mode,
    pattern: RegexTree,
}

impl SingleLineCommand<ClassDiagram> for CreateElementFull2 {
    fn pattern(&self) -> &RegexTree {
        &self.pattern
    }

    fn is_forbidden(&self, line: &str) -> bool {
        static FORBIDDEN: LazyLock<Regex> = LazyLock::new(|| java_regex(r"^[\p{L}0-9_.]+$", false));
        FORBIDDEN.is_match(line)
    }

    fn execute_arg(
        &self,
        diagram: &mut ClassDiagram,
        _: &LineLocation,
        _: &RegexResult,
    ) -> CommandResult {
        if self.mode == Mode::NormalKeyword && !diagram.allow_mixing {
            return Err(CommandError::new(
                "Use 'allowmixing' if you want to mix classes and other UML elements.",
            ));
        }
        diagram.command_not_ported("CommandCreateElementFull2");
        Ok(())
    }
}

/// PlantUML's `CommandNamespace`.
pub(super) fn namespace<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandNamespace",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"namespace"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "NAME", r"([%pLN_][-%pLN_.:\\/]*)"),
            stereo::optional_pattern("STEREOTYPE"),
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

/// PlantUML's `CommandNamespace2`.
pub(super) fn namespace2<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandNamespace2",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"namespace"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"[%g]"),
            RegexTree::named(1, "DISPLAY", r"([^%g]+)"),
            RegexTree::leaf(r"[%g]"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"as"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "NAME", r"([%pLN_][-%pLN_.:\\/]*)"),
            stereo::optional_pattern("STEREOTYPE"),
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

/// PlantUML's `CommandNamespaceEmpty`.
pub(super) fn namespace_empty<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandNamespaceEmpty",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"namespace"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "NAME", r"([%pLN_][-%pLN_.:\\/]*)"),
            stereo::optional_pattern("STEREOTYPE"),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\}"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandStereotype`.
pub(super) fn stereotype<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandStereotype",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "NAME", r"([%pLN_.]+|[%g][^%g]+[%g])"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "STEREO", r"(\<\<.+?\>\>)"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandLinkClass`.
pub(super) fn link_class<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandLinkClass",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "HEADER", r"@([\d.]+)"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::or(vec![
                RegexTree::named(1, "ENT1", r"((?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_$]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_$]+)*|[%g][^%g]+[%g])"),
                RegexTree::named(2, "COUPLE1", r"\([%s]*((?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_]+)*|[%g](?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_]+)*[%g]))[%s]*,[%s]*((?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_]+)*|[%g](?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_]+)*[%g]))[%s]*\)"),
            ]),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"[\[]"),
                RegexTree::named(1, "QUALIFIER1", r"([^\[\]]+)"),
                RegexTree::leaf(r"[\]]"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::optional(RegexTree::named(1, "FIRST_LABEL", r"[%g]([^%g]+)[%g]")),
            RegexTree::optional(RegexTree::named(1, "FIRST_ROLE", r"/([^%s]+|[%g][^%g]+[%g])")),
            RegexTree::spaces_zero_or_more(),
            RegexTree::concat(vec![
                RegexTree::named(1, "ARROW_HEAD1", r"(\Q<|:\E|\Q<||\E|\Q<|\E|\Q||\E|\Q<<\E|\Q<_\E|\Q}o\E\b|\Q|o\E\b|\Q0)\E|\Q}|\E|\Q@\E|\Q#\E|\Q)\E|\Q*\E|\Q+\E|\b\Qo\E\b|\Q0\E|\Qx\E|\Q<\E|\Q}\E|\Q^\E)?"),
                RegexTree::named(1, "ARROW_BODY1", r"([-=.]+)"),
                RegexTree::named(1, "ARROW_STYLE1", r"(?:\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)\])?"),
                RegexTree::named(1, "ARROW_DIRECTION", r"(left|right|up|down|le?|ri?|up?|do?)?"),
                RegexTree::optional(RegexTree::named(1, "INSIDE", r"(0|\(0\)|\(0|0\))(?=[-=.~])")),
                RegexTree::named(1, "ARROW_STYLE2", r"(?:\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)\])?"),
                RegexTree::named(1, "ARROW_BODY2", r"([-=.]*)"),
                RegexTree::named(1, "ARROW_HEAD2", r"(\Q:|>\E|\Q||>\E|\Q||\E|\Q>>\E|\Q\\\E|\Q//\E|\Q|>\E|\Q(0\E|\b\Qo{\E|\b\Qo|\E|\Q|{\E|\Q_>\E|\Q@\E|\Q#\E|\Q(\E|\Q*\E|\Q+\E|\b\Qo\E\b|\Q0\E|\Qx\E|\Q{\E|\Q^\E|\Q>\E)?"),
            ]),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(1, "SECOND_LABEL", r"[%g]([^%g]+)[%g]")),
            RegexTree::optional(RegexTree::named(1, "SECOND_ROLE", r"/([^%s]+|[%g][^%g]+[%g])")),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"[\[]"),
                RegexTree::named(1, "QUALIFIER2", r"([^\[\]]+)"),
                RegexTree::leaf(r"[\]]"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::or(vec![
                RegexTree::named(1, "ENT2", r"((?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_$]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_$]+)*|[%g][^%g]+[%g])"),
                RegexTree::named(2, "COUPLE2", r"\([%s]*((?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_]+)*|[%g](?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_]+)*[%g]))[%s]*,[%s]*((?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_]+)*|[%g](?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_]+)*[%g]))[%s]*\)"),
            ]),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            stereo::optional_pattern("STEREOTYPE"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r":"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "LABEL_LINK", r"(.+)"),
            ])),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandLinkLollipop`.
pub(super) fn link_lollipop<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandLinkLollipop",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "HEADER", r"@([\d.]+)"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(3, "ENT1", r"(?:(interface|enum|annotation|abstract[%s]+class|abstract|class|entity|protocol|struct|exception|metaclass|stereotype|dataclass|record)[%s]+)?((?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_$]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_$]+)*|[%g][^%g]+[%g])[%s]*(\<\<.*\>\>)?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(1, "FIRST_LABEL", r"[%g]([^%g]+)[%g]")),
            RegexTree::spaces_zero_or_more(),
            RegexTree::or(vec![
                RegexTree::named(2, "LOL_THEN_ENT", r"([()]\))([-=.]+)"),
                RegexTree::named(2, "ENT_THEN_LOL", r"([-=.]+)(\([()])"),
            ]),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(1, "SECOND_LABEL", r"[%g]([^%g]+)[%g]")),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(3, "ENT2", r"(?:(interface|enum|annotation|abstract[%s]+class|abstract|class|entity|protocol|struct|exception|metaclass|stereotype|dataclass|record)[%s]+)?((?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_$]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_$]+)*|[%g][^%g]+[%g])[%s]*(\<\<.*\>\>)?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r":"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "LABEL_LINK", r"(.+)"),
            ])),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandFactoryTipOnEntity$1`.
pub(super) fn tip_on_entity_multi_line_with_bracket<D: NotPortedCommands + 'static>()
-> Box<dyn Command<D>> {
    Box::new(unported::multi_line(
        "CommandFactoryTipOnEntity$1",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"note"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "POSITION", r"(right|left)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"of"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(
                2,
                "CODE",
                r"([^%s{}%g<>:]+|[%g][^%g]+[%g])::([%g][^%g]+[%g]|[^%s]+)",
            ),
            RegexTree::spaces_zero_or_more(),
            stereo::tags_pattern("TAGS1"),
            stereo::optional_pattern("STEREO"),
            stereo::tags_pattern("TAGS2"),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{"),
            RegexTree::end(),
        ]),
        &plantuml_regex(r"^(\})$"),
    ))
}

/// PlantUML's `CommandFactoryTipOnEntity$1`.
pub(super) fn tip_on_entity_multi_line<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    Box::new(unported::multi_line(
        "CommandFactoryTipOnEntity$1",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"note"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "POSITION", r"(right|left)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"of"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(
                2,
                "CODE",
                r"([^%s{}%g<>:]+|[%g][^%g]+[%g])::([%g][^%g]+[%g]|[^%s]+)",
            ),
            RegexTree::spaces_zero_or_more(),
            stereo::tags_pattern("TAGS1"),
            stereo::optional_pattern("STEREO"),
            stereo::tags_pattern("TAGS2"),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            Url::optional_pattern(),
            RegexTree::end(),
        ]),
        &plantuml_regex(r"^[%s]*(end[%s]?note)$"),
    ))
}

/// PlantUML's `CommandConstraintOnLinks`.
pub(super) fn constraint_on_links<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandConstraintOnLinks",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"constraint"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"on"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"links"),
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

/// PlantUML's `CommandDiamondAssociation`.
pub(super) fn diamond_association<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandDiamondAssociation",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"\<\>"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "CODE", r"([%pLN_.]+)"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}
