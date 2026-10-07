//! The commands of usecase, component and deployment diagrams, in the order
//! `DescriptionDiagramFactory` tries them. None is ported yet: they only recognise their lines.

use crate::command::Command;
use crate::command::unported::{self, NotPortedCommands};
use crate::klimt::url::Url;
use crate::pattern::{RegexTree, plantuml_regex};
use crate::{color, stereo};

/// PlantUML's `CommandLinkElement`.
pub(super) fn link_element<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandLinkElement",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "ENT1", r"([%pLN_.]+|[%g][^%g]+[%g]|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|(?!\[\*\])\[[^\[\]]+\]|\((?!\*\))[^)]+\)/?)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(1, "FIRST_LABEL", r"[%g]([^%g]+)[%g]")),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "HEAD1", r"(\Q<|:\E|\Q<||\E|\Q<|\E|\Q||\E|\Q<<\E|\Q<_\E|\Q}o\E\b|\Q|o\E\b|\Q0)\E|\Q}|\E|\Q@\E|\Q#\E|\Q)\E|\Q*\E|\Q+\E|\b\Qo\E\b|\Q0\E|\Qx\E|\Q<\E|\Q}\E|\Q^\E)?"),
            RegexTree::named(1, "BODY1", r"([-=.~]+)"),
            RegexTree::named(1, "ARROW_STYLE1", r"(?:\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*(?:(?:;(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)*))\])?"),
            RegexTree::optional(RegexTree::named(1, "DIRECTION", r"(left|right|up|down|le?|ri?|up?|do?)(?=[-=.~0()\[])")),
            RegexTree::optional(RegexTree::named(1, "INSIDE", r"(0|\(0\)|\(0|0\))(?=[-=.~])")),
            RegexTree::named(1, "ARROW_STYLE2", r"(?:\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)\])?"),
            RegexTree::named(1, "BODY2", r"([-=.~]*)"),
            RegexTree::named(1, "HEAD2", r"(\Q:|>\E|\Q||>\E|\Q||\E|\Q>>\E|\Q\\\E|\Q//\E|\Q|>\E|\Q(0\E|\b\Qo{\E|\b\Qo|\E|\Q|{\E|\Q_>\E|\Q@\E|\Q#\E|\Q(\E|\Q*\E|\Q+\E|\b\Qo\E\b|\Q0\E|\Qx\E|\Q{\E|\Q^\E|\Q>\E)?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(1, "SECOND_LABEL", r"[%g]([^%g]+)[%g]")),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "ENT2", r"([%pLN_.]+|[%g][^%g]+[%g]|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|(?!\[\*\])\[[^\[\]]+\]|\((?!\*\))[^)]+\)/?)"),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            stereo::optional_pattern("STEREOTYPE"),
            RegexTree::named(1, "LABEL_LINK", r"(?::[%s]*(.+))?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandCreateElementFull`.
pub(super) fn create_element_full<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandCreateElementFull",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "SYMBOL", r"(?:(person|artifact|actor/|actor|folder|card|file|package|rectangle|hexagon|label|node|frame|cloud|action|process|database|queue|stack|storage|agent|usecase/|usecase|component|boundary|control|entity|interface|circle|collections|port|portin|portout|\(\))[%s]+)?"),
            color::optional_pattern("COLOR2"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::or(vec![
                RegexTree::named(1, "CODE1", r"([%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\]|[%g].+?[%g])"),
                RegexTree::concat(vec![
                    RegexTree::named(1, "DISPLAY2", r"([%g].+?[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
                    stereo::optional_pattern("STEREOTYPE2"),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::named(1, "CODE2", r"([%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::named(1, "CODE3", r"([%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
                    stereo::optional_pattern("STEREOTYPE3"),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::named(1, "DISPLAY3", r"([%g].+?[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::named(1, "DISPLAY4", r"([%g].+?[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\]|[%pLN_.]+)"),
                    stereo::optional_pattern("STEREOTYPE4"),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::named(1, "CODE4", r"([%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
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
        ]),
    )
        .forbidding(r"[\p{L}0-9_.]+")
    .boxed()
}

/// PlantUML's `CommandArchimate`.
pub(super) fn archimate<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandArchimate",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(0, "SYMBOL", r"archimate"),
            RegexTree::spaces_one_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_one_or_more(),
            RegexTree::or(vec![
                RegexTree::named(1, "CODE1", r"([%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\]|[%g].+?[%g])"),
                RegexTree::concat(vec![
                    RegexTree::named(1, "DISPLAY2", r"([%g].+?[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
                    RegexTree::concat(vec![
                        RegexTree::spaces_zero_or_more(),
                        RegexTree::optional(RegexTree::named(1, "STEREOTYPE2", r"(\<\<[-\w]+?\>\>)")),
                        RegexTree::spaces_zero_or_more(),
                    ]),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::named(1, "CODE2", r"([%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::named(1, "CODE3", r"([%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
                    RegexTree::concat(vec![
                        RegexTree::spaces_zero_or_more(),
                        RegexTree::optional(RegexTree::named(1, "STEREOTYPE3", r"(\<\<[-\w]+?\>\>)")),
                        RegexTree::spaces_zero_or_more(),
                    ]),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::named(1, "DISPLAY3", r"([%g].+?[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::named(1, "DISPLAY4", r"([%g].+?[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\]|[%pLN_.]+)"),
                    RegexTree::concat(vec![
                        RegexTree::spaces_zero_or_more(),
                        RegexTree::optional(RegexTree::named(1, "STEREOTYPE4", r"(\<\<[-\w]+?\>\>)")),
                        RegexTree::spaces_zero_or_more(),
                    ]),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::named(1, "CODE4", r"([%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
                ]),
            ]),
            RegexTree::concat(vec![
                RegexTree::spaces_zero_or_more(),
                RegexTree::optional(RegexTree::named(1, "STEREOTYPE", r"(\<\<[-\w]+?\>\>)")),
                RegexTree::spaces_zero_or_more(),
            ]),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandArchimateMultilines`.
pub(super) fn archimate_multilines<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    Box::new(
        unported::multi_line(
            "CommandArchimateMultilines",
            RegexTree::concat(vec![
                RegexTree::start(),
                RegexTree::leaf(r"archimate"),
                RegexTree::spaces_one_or_more(),
                color::optional_pattern("COLOR"),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(1, "CODE", r"([%pLN_.]+)"),
                RegexTree::concat(vec![
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::optional(RegexTree::named(1, "STEREOTYPE", r"(\<\<[-\w]+?\>\>)")),
                    RegexTree::spaces_zero_or_more(),
                ]),
                Url::optional_pattern(),
                RegexTree::spaces_zero_or_more(),
                color::optional_pattern("COLOR"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::leaf(r"\["),
                RegexTree::named(1, "DESC", r"(.*)"),
                RegexTree::end(),
            ]),
            &plantuml_regex(r"^(.*)\]$"),
        )
        .skipping_quote_lines(),
    )
}

/// PlantUML's `CommandArchimatePackage`.
pub(super) fn archimate_package<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandArchimatePackage",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(0, "SYMBOL", r"archimate"),
            RegexTree::spaces_one_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_one_or_more(),
            RegexTree::or(vec![
                RegexTree::named(1, "CODE1", r"([%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\]|[%g].+?[%g])"),
                RegexTree::concat(vec![
                    RegexTree::named(1, "DISPLAY2", r"([%g].+?[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
                    RegexTree::concat(vec![
                        RegexTree::spaces_zero_or_more(),
                        RegexTree::optional(RegexTree::named(1, "STEREOTYPE2", r"(\<\<[-\w]+?\>\>)")),
                        RegexTree::spaces_zero_or_more(),
                    ]),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::named(1, "CODE2", r"([%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::named(1, "CODE3", r"([%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
                    RegexTree::concat(vec![
                        RegexTree::spaces_zero_or_more(),
                        RegexTree::optional(RegexTree::named(1, "STEREOTYPE3", r"(\<\<[-\w]+?\>\>)")),
                        RegexTree::spaces_zero_or_more(),
                    ]),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::named(1, "DISPLAY3", r"([%g].+?[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::named(1, "DISPLAY4", r"([%g].+?[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\]|[%pLN_.]+)"),
                    RegexTree::concat(vec![
                        RegexTree::spaces_zero_or_more(),
                        RegexTree::optional(RegexTree::named(1, "STEREOTYPE4", r"(\<\<[-\w]+?\>\>)")),
                        RegexTree::spaces_zero_or_more(),
                    ]),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::named(1, "CODE4", r"([%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])"),
                ]),
            ]),
            RegexTree::concat(vec![
                RegexTree::spaces_zero_or_more(),
                RegexTree::optional(RegexTree::named(1, "STEREOTYPE", r"(\<\<[-\w]+?\>\>)")),
                RegexTree::spaces_zero_or_more(),
            ]),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// PlantUML's `CommandCreateDomain`.
pub(super) fn create_domain<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandCreateDomain",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "TYPE", r"(requirement|domain)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(2, "DISPLAY", r"[%g](.+?)(?:\<([^\<\>/](?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<(?:[^\<\>/]|\<\>)*\>)*\>)*\>)*\>)*)\>)?[%g]"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"as"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "CODE", r"([a-zA-Z0-9]+)"),
            stereo::optional_pattern("STEREO"),
            RegexTree::named(1, "GROUP", r"(\{)?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}
