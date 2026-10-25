//! The commands of legacy activity diagrams (PlantUML's `activitydiagram.command` package and the commands
//! `ActivityDiagramFactory` takes from elsewhere).

use std::sync::LazyLock;

use regex::Regex;

use super::ActivityDiagram;
use crate::abel::{EntityId, GroupType, LeafType, LinkArg, Position};
use crate::color::{self, ColorType, Colors, HColor};
use crate::command::{
    BlocLines, Command, CommandError, CommandResult, Multiline, ParserPass, PatternCommand,
    SingleLine, UBrexSingleLine,
};
use crate::creole::Display;
use crate::decoration::{LinkDecor, LinkType, WithLinkType};
use crate::diagram::common_commands::add_common_commands1;
use crate::diagram::cuca::EntityDiagram;
use crate::diagram::cuca_commands::note;
use crate::diagram::description::arrow_style;
use crate::direction::Direction;
use crate::java;
use crate::jaws::BLOCK_E1_NEWLINE;
use crate::klimt::url::Url;
use crate::pattern::{RegexResult, RegexTree, plantuml_regex};
use crate::skin::Rankdir;
use crate::stereo::{self, Stereotype};
use crate::text::{LineLocation, unquoted};
use crate::ubrex::UMatcher;
use crate::ubrex::builder::UBrexPart;

/// `ActivityDiagramFactory.initCommandsList`, which leaves concurrent inner activities out.
pub(super) fn init_commands_list() -> Vec<Box<dyn Command<ActivityDiagram>>> {
    let mut commands = vec![footbox_ignored()];
    commands.extend(add_common_commands1());
    commands.extend([
        rank_dir(),
        partition(),
        end_partition(),
        link_long_activity(),
        note_activity(),
        note_activity_multi_line(),
        note::note_on_link(ParserPass::One),
        note::note_on_link_multi_line(ParserPass::One),
        if_command(),
        else_command(),
        endif(),
        link_activity(),
        hide_show2(),
    ]);
    commands
}

fn ubrex_command(
    parts: Vec<UBrexPart>,
    apply: fn(&mut ActivityDiagram, &LineLocation, &UMatcher) -> CommandResult,
) -> Box<dyn Command<ActivityDiagram>> {
    let mut parts = parts;
    parts.push(UBrexPart::end());
    Box::new(UBrexSingleLine::new(
        UBrexPart::concat(parts).build(),
        apply,
    ))
}

/// The first value `key` captured (`RegexResult.get(key, 0)` on a match).
fn value<'a>(matcher: &UMatcher<'a>, key: &str) -> Option<&'a str> {
    matcher.find_values_by_key(key).first().copied()
}

/// The first value captured under a key starting with `prefix` (`RegexResult.getLazzy(prefix, 0)`).
fn lazzy<'a>(matcher: &UMatcher<'a>, prefix: &str) -> Option<&'a str> {
    matcher
        .find_first_values_by_key_prefix(prefix)
        .first()
        .copied()
}

/// PlantUML's `UBrexCommandFootboxIgnored`: `hide footbox` means nothing here.
fn footbox_ignored() -> Box<dyn Command<ActivityDiagram>> {
    ubrex_command(
        vec![
            UBrexPart::leaf("【 hide┇show 】"),
            UBrexPart::space_zero_or_more(),
            UBrexPart::leaf("footbox"),
        ],
        |_, _, _| Ok(()),
    )
}

/// PlantUML's `UBrexCommandRankDir`: `left to right direction` lays the diagram out sideways.
fn rank_dir() -> Box<dyn Command<ActivityDiagram>> {
    ubrex_command(
        vec![
            UBrexPart::named(
                "DIRECTION",
                UBrexPart::leaf("【 left∙to∙right ┇ top∙to∙bottom 】"),
            ),
            UBrexPart::space_one_or_more(),
            UBrexPart::leaf("direction"),
        ],
        |diagram, _, matcher| {
            let direction = value(matcher, "DIRECTION").unwrap_or_default();
            let rankdir = if direction.to_ascii_lowercase().starts_with("left") {
                Rankdir::LeftToRight
            } else {
                Rankdir::TopToBottom
            };
            diagram.cuca.titled.skin.set_rankdir(rankdir);
            Ok(())
        },
    )
}

/// PlantUML's `UBrexCommandHideShow2`: `hide Foo`, `show <<Stereo>>`.
fn hide_show2() -> Box<dyn Command<ActivityDiagram>> {
    ubrex_command(
        vec![
            UBrexPart::named(
                "COMMAND",
                UBrexPart::leaf("【hide-class┇hide┇show-class┇show】"),
            ),
            UBrexPart::space_one_or_more(),
            UBrexPart::named("WHAT", UBrexPart::leaf("【 << 〇*「〤<>」>> ┇ 〇+〴S 】 ")),
        ],
        |diagram, _, matcher| {
            let show =
                value(matcher, "COMMAND").is_some_and(|command| command.starts_with(['s', 'S']));
            let what = java::trim(value(matcher, "WHAT").unwrap_or_default());
            diagram.cuca.hide_or_show2(what, show);
            Ok(())
        },
    )
}

/// PlantUML's `UBrexCommandPartition`: `partition Name #color <<stereo>> {`.
fn partition() -> Box<dyn Command<ActivityDiagram>> {
    ubrex_command(
        vec![
            UBrexPart::leaf("partition"),
            UBrexPart::space_one_or_more(),
            UBrexPart::leaf("【 〃 〶$NAME=〇+「〤〃」 〃 ┇ 〶$NAME=〇+〴S   】"),
            UBrexPart::space_zero_or_more(),
            UBrexPart::or(vec![
                // UBrexColorParser.simpleColor(ColorType.BACK)
                UBrexPart::named(
                    "COLOR",
                    UBrexPart::leaf("# 〇+〴w 〇?〘 「-\\|/」 〇+〴w 〙"),
                ),
                // A legacy colour, ignored.
                UBrexPart::optional(UBrexPart::leaf(
                    "【 # 〇{6}「0〜9a〜fA〜F」┇ 〇?# 〇+〴w 】",
                )),
            ]),
            UBrexPart::optional(UBrexPart::named(
                "STEREOTYPE",
                UBrexPart::leaf("<<  〄+〴. ->〘 >>〙"),
            )),
            UBrexPart::space_zero_or_more(),
            UBrexPart::optional(UBrexPart::leaf("{")),
        ],
        |diagram, location, matcher| {
            let name = value(matcher, "NAME").unwrap_or_default();
            let cuca = &mut diagram.cuca;
            let quark = cuca.quark_in_context(true, ActivityDiagram::clean_id(name))?;
            let display = Display::with_newlines(cuca.quark(quark).get_name());
            cuca.goto_group(Some(location), quark, display, GroupType::Package);
            let colors = lazzy(matcher, "COLOR")
                .map(|data| Colors::parse(data, ColorType::Back))
                .transpose()?
                .unwrap_or_default();
            let group = cuca.get_current_group();
            let p = cuca.entity_mut(group);
            if !colors.is_empty() {
                p.colors = colors;
            }
            p.stereotype = value(matcher, "STEREOTYPE").map(Stereotype::new);
            Ok(())
        },
    )
}

/// PlantUML's `UBrexCommandEndPartition`: `end partition` or `}`.
fn end_partition() -> Box<dyn Command<ActivityDiagram>> {
    ubrex_command(
        vec![UBrexPart::or(vec![
            UBrexPart::concat(vec![
                UBrexPart::leaf("end"),
                UBrexPart::space_zero_or_more(),
                UBrexPart::leaf("partition"),
            ]),
            UBrexPart::leaf("}"),
        ])],
        |diagram, _, _| {
            if diagram.cuca.end_group() {
                Ok(())
            } else {
                Err(CommandError::new("No partition defined"))
            }
        },
    )
}

/// `CommandLinkElement.UBREX_LINE_STYLE`: the styles of a link, like `#red,dashed`.
const UBREX_LINE_STYLE: &str = "【 #〇+〴w ┇dotted┇dashed┇plain┇bold┇hidden┇norank┇single┇node┇thickness=〇+〴d】 〇*【 ,#〇+〴w ┇,dotted┇,dashed┇,plain┇,bold┇,hidden┇,norank┇,single┇,node┇,thickness=〇+〴d】";

/// PlantUML's `UBrexCommandIf`: `if "test" then`, after an optional source and arrow, opens a branch.
fn if_command() -> Box<dyn Command<ActivityDiagram>> {
    let simple = || {
        UBrexPart::concat(vec![
            UBrexPart::space_zero_or_more(),
            UBrexPart::optional(UBrexPart::concat(vec![
                UBrexPart::named("ARROW_BODY1", UBrexPart::leaf("〇+「-.」")),
                UBrexPart::leaf(&format!(
                    "〇?〘 [ 〶$ARROW_STYLE1=〘{UBREX_LINE_STYLE}〙] 〙"
                )),
                UBrexPart::named(
                    "ARROW_DIRECTION",
                    UBrexPart::leaf("〇?【 *┇left┇right┇up┇down┇l〇?e┇r〇?i┇u〇?p┇d〇?o】"),
                ),
                UBrexPart::leaf(&format!(
                    "〇?〘 [  〶$ARROW_STYLE2=〘{UBREX_LINE_STYLE}〙] 〙"
                )),
                UBrexPart::named("ARROW_BODY2", UBrexPart::leaf("〇*「-.」")),
                UBrexPart::leaf(">"),
            ])),
            UBrexPart::space_zero_or_more(),
            UBrexPart::optional(UBrexPart::leaf(
                "[ 〶$BRACKET=〘〇+「〤]*」  〇*「〤]」〙 ]",
            )),
            UBrexPart::space_zero_or_more(),
            UBrexPart::or(vec![
                UBrexPart::leaf(
                    "if 〇*〴s 〴g  〶$IF1=〇*〴G 〴g 〇*〴s 〇?〘as 〇+〴s 〶$ASIF1=〇+「〴an_.」 〇+〴s 〙〘then〙",
                ),
                UBrexPart::leaf("if 〇+〴s 〶$IF2=〇l+〴. 〘then〙 "),
                UBrexPart::leaf(
                    "if 〇*〴s 〴g  〶$IF1=〇*〴G 〴g 〇*〴s 〇?〘as 〇+〴s 〶$ASIF1=〇+「〴an_.」 〇+〴s 〙",
                ),
                UBrexPart::leaf("if 〇+〴s 〶$IF2=〇+〴."),
            ]),
        ])
    };
    ubrex_command(
        vec![UBrexPart::or(vec![
            simple(),
            UBrexPart::concat(vec![
                UBrexPart::named("STAR", UBrexPart::leaf("(* 〇?〘top〙)")),
                simple(),
            ]),
            UBrexPart::concat(vec![
                UBrexPart::named(
                    "BAR",
                    UBrexPart::leaf("=〇+= 〇*〴s 〇+「〴an_.」〇*〴s =〇+="),
                ),
                simple(),
            ]),
            UBrexPart::concat(vec![
                UBrexPart::leaf(
                    "〴g 〶$QUOTED1=〇*〴G 〴g 〇?〘〇+〴s as 〇+〴s 〶$QUOTED2=〇+「〴an_.」 〙 ",
                ),
                simple(),
            ]),
            UBrexPart::concat(vec![
                UBrexPart::named("CODE", UBrexPart::leaf("〇+「〴an_.」")),
                simple(),
            ]),
        ])],
        execute_if,
    )
}

fn execute_if(
    diagram: &mut ActivityDiagram,
    location: &LineLocation,
    arg: &UMatcher,
) -> CommandResult {
    let entity1 = ubrex_get_entity_for_if_only(location, diagram, arg)?
        .ok_or_else(|| CommandError::new("No if possible at this point"))?;
    let (if_code, if_label) = match value(arg, "IF2") {
        None => (value(arg, "ASIF1"), value(arg, "IF1")),
        Some(if2) => (None, Some(if2)),
    };
    diagram.start_if(location, if_code)?;
    let length = if value(arg, "ARROW_BODY1").is_some() {
        arrow_length(
            value(arg, "ARROW_BODY1"),
            value(arg, "ARROW_DIRECTION"),
            value(arg, "ARROW_BODY2"),
        )
    } else {
        2
    };
    let branch = diagram.current_branch().expect("an if was just started");
    let link_arg = LinkArg::build(value(arg, "BRACKET").map(Display::with_newlines), length)
        .with_quantifier(None, if_label.map(str::to_owned));
    let cuca = &mut diagram.cuca;
    let link = cuca.new_link(
        Some(location),
        entity1,
        branch,
        LinkType::new(LinkDecor::Arrow, LinkDecor::None),
        link_arg,
    );
    cuca.link_mut(link).apply_style(lazzy(arg, "ARROW_STYLE"));
    cuca.add_link(link);
    Ok(())
}

/// What an `if` starts from: the element written before its arrow, or else the latest one
/// (`CommandLinkActivity.ubrexGetEntityForIfOnly`).
fn ubrex_get_entity_for_if_only(
    location: &LineLocation,
    diagram: &mut ActivityDiagram,
    arg: &UMatcher,
) -> Result<Option<EntityId>, CommandError> {
    if value(arg, "STAR").is_some() {
        return diagram.get_start(location).map(Some);
    }
    if let Some(id_short) = value(arg, "CODE") {
        return activity(diagram, location, id_short, id_short).map(Some);
    }
    if let Some(bar) = value(arg, "BAR") {
        return synchro_bar(diagram, location, bar).map(Some);
    }
    if let Some(quoted1) = value(arg, "QUOTED1") {
        let quoted_string = value(arg, "QUOTED2").unwrap_or(quoted1);
        return activity(diagram, location, quoted_string, quoted1).map(Some);
    }
    Ok(diagram.last_entity_consulted)
}

/// PlantUML's `UBrexCommandElse`: the next links start from the branch of the innermost `if`.
fn else_command() -> Box<dyn Command<ActivityDiagram>> {
    ubrex_command(vec![UBrexPart::leaf("else")], |diagram, _, _| {
        let branch = diagram
            .last_entity_consulted
            .and(diagram.current_branch())
            .ok_or_else(|| CommandError::new("No if for this else"))?;
        diagram.last_entity_consulted = Some(branch);
        Ok(())
    })
}

/// PlantUML's `UBrexCommandEndif`.
fn endif() -> Box<dyn Command<ActivityDiagram>> {
    ubrex_command(
        vec![
            UBrexPart::leaf("end"),
            UBrexPart::space_zero_or_more(),
            UBrexPart::leaf("if"),
        ],
        |diagram, _, _| {
            if diagram.last_entity_consulted.is_none() || diagram.current_branch().is_none() {
                return Err(CommandError::new("No if for this endif"));
            }
            diagram.conditional_contexts.pop();
            Ok(())
        },
    )
}

/// What may start a link: `(*)`, a name, a bar `===B1===` or a quoted text, maybe `as` a name.
fn link_source() -> RegexTree {
    RegexTree::optional(RegexTree::named_or(
        "FIRST",
        vec![
            RegexTree::named(2, "STAR", r"(\(\*(top)?\))"),
            RegexTree::named(1, "CODE", r"([%pLN][%pLN_.]*)"),
            RegexTree::named(1, "BAR", r"(?:==+)[%s]*([%pLN_.]+)[%s]*(?:==+)"),
            RegexTree::named(2, "QUOTED", r"[%g]([^%g]+)[%g](?:[%s]+as[%s]+([%pLN_.]+))?"),
        ],
    ))
}

/// The arrow of a link, like `-->`, `-left->` or `-[#red]->`.
fn link_arrow() -> Vec<RegexTree> {
    vec![
        RegexTree::named(1, "ARROW_BODY1", r"([-.]+)"),
        RegexTree::named(1, "ARROW_STYLE1", arrow_style()),
        RegexTree::named(
            1,
            "ARROW_DIRECTION",
            r"(\*|left|right|up|down|le?|ri?|up?|do?)?",
        ),
        RegexTree::named(1, "ARROW_STYLE2", arrow_style()),
        RegexTree::named(1, "ARROW_BODY2", r"([-.]*)"),
        RegexTree::leaf(r"\>"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::optional(RegexTree::named(1, "BRACKET", r"\[([^\]*]+[^\]]*)\]")),
        RegexTree::spaces_zero_or_more(),
    ]
}

/// PlantUML's `CommandLinkActivity`: `A --> B`, creating the ends it names.
fn link_activity() -> Box<dyn Command<ActivityDiagram>> {
    let mut pattern = vec![
        RegexTree::start(),
        link_source(),
        stereo::optional_pattern("STEREOTYPE"),
        color::exp2(),
        RegexTree::spaces_zero_or_more(),
        Url::optional_pattern(),
    ];
    pattern.extend(link_arrow());
    pattern.extend([
        RegexTree::named_or(
            "FIRST2",
            vec![
                RegexTree::named(2, "STAR2", r"(\(\*(top|\d+)?\))"),
                RegexTree::named(1, "OPENBRACKET2", r"(\{)"),
                RegexTree::named(1, "CODE2", r"([%pLN][%pLN_.]*)"),
                RegexTree::named(1, "BAR2", r"(?:==+)[%s]*([%pLN_.]+)[%s]*(?:==+)"),
                RegexTree::named(
                    2,
                    "QUOTED2",
                    r"[%g]([^%g]+)[%g](?:[%s]+as[%s]+([%pLN][%pLN_.]*))?",
                ),
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
        color::exp3(),
        RegexTree::end(),
    ]);
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(pattern),
        execute_link_activity,
    )))
}

fn execute_link_activity(
    diagram: &mut ActivityDiagram,
    location: &LineLocation,
    arg: &RegexResult,
) -> CommandResult {
    let no_such_activity = || CommandError::new("No such activity");
    let entity1 = get_entity(location, diagram, arg, true)?.ok_or_else(no_such_activity)?;
    set_stereotype_and_color(
        diagram,
        entity1,
        arg.get("STEREOTYPE", 0),
        arg.get("BACKCOLOR", 0),
    )?;
    let entity2 = get_entity(location, diagram, arg, false)?.ok_or_else(no_such_activity)?;
    diagram.last_entity_consulted = Some(entity2);
    set_stereotype_and_color(
        diagram,
        entity2,
        arg.get("STEREOTYPE2", 0),
        arg.get("BACKCOLOR2", 0),
    )?;
    let label = arg.get("BRACKET", 0).map(Display::with_newlines);
    let (body1, direction, body2) = (
        arg.get("ARROW_BODY1", 0),
        arg.get("ARROW_DIRECTION", 0),
        arg.get("ARROW_BODY2", 0),
    );
    let unconstrained = direction.is_some_and(|direction| direction.contains('*'));
    let length = if unconstrained {
        2
    } else {
        arrow_length(body1, direction, body2)
    };
    add_activity_link(
        diagram,
        location,
        arg,
        entity1,
        entity2,
        label,
        length,
        unconstrained,
    );
    Ok(())
}

/// Links `entity1` to `entity2` with the arrow `arg` describes, drawn from its other end when it points
/// left or up.
#[expect(
    clippy::too_many_arguments,
    reason = "the parts of PlantUML's two link commands"
)]
fn add_activity_link(
    diagram: &mut ActivityDiagram,
    location: &LineLocation,
    arg: &RegexResult,
    entity1: EntityId,
    entity2: EntityId,
    label: Option<Display>,
    length: i32,
    unconstrained: bool,
) {
    let (body1, direction, body2) = (
        arg.get("ARROW_BODY1", 0).unwrap_or_default(),
        arg.get("ARROW_DIRECTION", 0).unwrap_or_default(),
        arg.get("ARROW_BODY2", 0).unwrap_or_default(),
    );
    let mut link_type = LinkType::new(LinkDecor::Arrow, LinkDecor::None);
    if format!("{body1}{body2}").contains('.') {
        link_type = link_type.go_dotted();
    }
    let cuca = &mut diagram.cuca;
    let link_arg =
        LinkArg::build_managing(label, length, cuca.skin().class_attribute_icon_size() > 0);
    let mut link = cuca.new_link(Some(location), entity1, entity2, link_type, link_arg);
    if unconstrained {
        cuca.link_mut(link).constraint = false;
    }
    let queue = format!("{body1}{direction}{body2}");
    if matches!(Direction::of_queue(&queue), Direction::Left | Direction::Up) {
        link = cuca.get_inv(link);
    }
    let link_mut = cuca.link_mut(link);
    if let Some(url) = arg.get("URL", 0) {
        link_mut.url = Url::parse(url);
    }
    link_mut.apply_style(arg.get_lazzy("ARROW_STYLE", 0));
    cuca.add_link(link);
}

/// How many ranks an arrow spans: its length, a sideways arrow counting as one dash and a vertical one as
/// at least two (`StringUtils.manageArrowForCuca`).
fn arrow_length(body1: Option<&str>, direction: Option<&str>, body2: Option<&str>) -> i32 {
    let queue = format!(
        "{}{}{}",
        body1.unwrap_or_default(),
        direction.unwrap_or_default(),
        body2.unwrap_or_default()
    );
    let dir = Direction::of_queue(&queue);
    let mut arrow: String = queue
        .replace('=', "-")
        .chars()
        .filter(|c| !(c.is_ascii_alphanumeric() || *c == '_'))
        .collect();
    arrow.push('>');
    if matches!(dir, Direction::Left | Direction::Right) {
        static DASHES: LazyLock<Regex> = LazyLock::new(|| Regex::new("-+").unwrap());
        arrow = DASHES.replace_all(&arrow, "-").into_owned();
    }
    if arrow.len() == 2 && matches!(dir, Direction::Up | Direction::Down) {
        arrow = arrow.replacen('-', "--", 1);
    }
    i32::try_from(arrow.len() - 1).expect("an arrow fits a line")
}

fn set_stereotype_and_color(
    diagram: &mut ActivityDiagram,
    entity: EntityId,
    stereotype: Option<&str>,
    back_color: Option<&str>,
) -> CommandResult {
    let entity = diagram.cuca.entity_mut(entity);
    if let Some(stereotype) = stereotype {
        entity.stereotype = Some(Stereotype::new(stereotype));
    }
    if let Some(color) = back_color {
        entity.colors = entity
            .colors
            .with(ColorType::Back, Some(parse_color(color)?));
    }
    Ok(())
}

fn parse_color(color: &str) -> Result<HColor, CommandError> {
    HColor::parse(color)
        .ok()
        .flatten()
        .ok_or_else(CommandError::bad_color)
}

/// The element one end of a link names, created if new; the latest one for a link without source, and
/// `None` when there is none (`CommandLinkActivity.getEntity`).
fn get_entity(
    location: &LineLocation,
    diagram: &mut ActivityDiagram,
    arg: &RegexResult,
    start: bool,
) -> Result<Option<EntityId>, CommandError> {
    let suf = if start { "" } else { "2" };
    let key = |name: &str| format!("{name}{suf}");
    if arg.get(&key("OPENBRACKET"), 0).is_some() {
        return diagram.create_inner_activity(location).map(Some);
    }
    if arg.get(&key("STAR"), 0).is_some() {
        return if start {
            diagram.get_start(location).map(Some)
        } else {
            diagram.get_end(location, arg.get("STAR2", 1)).map(Some)
        };
    }
    let partition = arg.get(&key("PARTITION"), 0).map(unquoted);
    if let Some(id_short) = arg.get(&key("CODE"), 0) {
        return in_partition(diagram, location, partition, true, |diagram| {
            activity(diagram, location, id_short, id_short)
        })
        .map(Some);
    }
    if let Some(bar) = arg.get(&key("BAR"), 0) {
        return synchro_bar(diagram, location, bar).map(Some);
    }
    if let Some(quoted) = arg.get(&key("QUOTED"), 0) {
        let quoted_string = arg.get(&key("QUOTED"), 1).unwrap_or(quoted);
        return in_partition(diagram, location, partition, false, |diagram| {
            activity(diagram, location, quoted_string, quoted)
        })
        .map(Some);
    }
    if let Some(invisible) = arg.get(&key("QUOTED_INVISIBLE"), 0) {
        return in_partition(diagram, location, partition, true, |diagram| {
            let quark = diagram
                .cuca
                .quark_in_context(true, ActivityDiagram::clean_id(invisible))?;
            let display = Display::with_newlines(diagram.cuca.quark(quark).get_name());
            Ok(diagram.leaf_named(location, quark, display, LeafType::Activity))
        })
        .map(Some);
    }
    if arg.get(&key("FIRST"), 0).is_none() {
        return Ok(diagram.last_entity_consulted);
    }
    Ok(None)
}

/// Runs `create` inside the partition `partition` names, if any. The partition shows its name as cleaned
/// when `display_cleaned`, else as written.
fn in_partition(
    diagram: &mut ActivityDiagram,
    location: &LineLocation,
    partition: Option<&str>,
    display_cleaned: bool,
    create: impl FnOnce(&mut ActivityDiagram) -> Result<EntityId, CommandError>,
) -> Result<EntityId, CommandError> {
    if let Some(partition) = partition {
        let cuca = &mut diagram.cuca;
        let quark = cuca.quark_in_context(true, ActivityDiagram::clean_id(partition))?;
        let display = if display_cleaned {
            Display::with_newlines(cuca.quark(quark).get_name())
        } else {
            Display::with_newlines(partition)
        };
        cuca.goto_group(Some(location), quark, display, GroupType::Package);
    }
    let result = create(diagram);
    if partition.is_some() {
        diagram.cuca.end_group();
    }
    result
}

/// The action named `id_short`, created showing `display` if new.
fn activity(
    diagram: &mut ActivityDiagram,
    location: &LineLocation,
    id_short: &str,
    display: &str,
) -> Result<EntityId, CommandError> {
    let quark = diagram
        .cuca
        .quark_in_context(true, ActivityDiagram::clean_id(id_short))?;
    Ok(diagram.leaf_named(
        location,
        quark,
        Display::with_newlines(display),
        LeafType::Activity,
    ))
}

/// The synchronization bar `===bar===`, created if new.
fn synchro_bar(
    diagram: &mut ActivityDiagram,
    location: &LineLocation,
    bar: &str,
) -> Result<EntityId, CommandError> {
    let quark = diagram
        .cuca
        .quark_in_context(true, ActivityDiagram::clean_id(bar))?;
    Ok(diagram.leaf_named(
        location,
        quark,
        Display::with_newlines(bar),
        LeafType::SynchroBar,
    ))
}

/// The last line of a long action: the end of its text, then maybe `as` a name, a stereotype, `in` a
/// partition and a colour (`CommandLinkLongActivity.END`).
static LONG_ACTIVITY_END: LazyLock<Regex> = LazyLock::new(|| {
    plantuml_regex(
        r"^[%s]*([^%g]*)[%g](?:[%s]+as[%s]+([%pLN][%pLN_.]*))?[%s]*(\<\<.*\>\>)?[%s]*(?:in[%s]+([%g][^%g]+[%g]|\S+))?[%s]*(#\w+)?$",
    )
});

fn link_long_activity_start() -> RegexTree {
    let mut pattern = vec![
        RegexTree::start(),
        link_source(),
        stereo::optional_pattern("STEREOTYPE"),
        RegexTree::named(1, "BACKCOLOR", r"(#\w+)?"),
        RegexTree::spaces_zero_or_more(),
        Url::optional_pattern(),
    ];
    pattern.extend(link_arrow());
    pattern.extend([
        RegexTree::leaf(r"[%g]"),
        RegexTree::named(1, "DESC", r"([^%g]*?)"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::end(),
    ]);
    RegexTree::concat(pattern)
}

/// PlantUML's `CommandLinkLongActivity`: a link to an action whose quoted text spans lines.
fn link_long_activity() -> Box<dyn Command<ActivityDiagram>> {
    let first_line = link_long_activity_start();
    Box::new(
        Multiline::starting_with_owned(
            link_long_activity_start(),
            &LONG_ACTIVITY_END,
            move |diagram: &mut ActivityDiagram, lines: &BlocLines| {
                execute_link_long_activity(diagram, &first_line, &lines.trimmed())
            },
        )
        .skipping_quote_lines(),
    )
}

fn execute_link_long_activity(
    diagram: &mut ActivityDiagram,
    first_line: &RegexTree,
    block: &BlocLines,
) -> CommandResult {
    let first = block.first().expect("a block has its first line");
    let location = first.location();
    let line0 = first_line
        .matcher(first.text())
        .expect("the block's first line matched");
    let entity1 = get_entity(location, diagram, &line0, true)?
        .ok_or_else(|| CommandError::new("No such entity"))?;
    set_stereotype_and_color(
        diagram,
        entity1,
        line0.get("STEREOTYPE", 0),
        line0.get("BACKCOLOR", 0),
    )?;

    let mut sb = String::new();
    let mut url_activity = None;
    if let Some(desc0) = line0.get("DESC", 0).filter(|desc0| !desc0.is_empty()) {
        url_activity = Url::parse(desc0);
        if url_activity.is_none() {
            sb.push_str(desc0);
            sb.push(BLOCK_E1_NEWLINE);
        }
    }
    let body = block.sub_extract(1, 1);
    for (i, cs) in body.iter().enumerate().map(|(i, cs)| (i + 1, cs)) {
        if i == 1 && url_activity.is_none() {
            url_activity = Url::parse(cs.text());
            if url_activity.is_some() {
                continue;
            }
        }
        sb.push_str(cs.text());
        if i < block.len() - 2 {
            sb.push(BLOCK_E1_NEWLINE);
        }
    }
    let last = block.last().expect("a block has its last line").text();
    let line_last = LONG_ACTIVITY_END
        .captures(last)
        .expect("the block's last line matched");
    let group = |index| line_last.get(index).map(|group| group.as_str());
    if let Some(end) = group(1).filter(|end| !end.is_empty()) {
        if !sb.is_empty() && !sb.ends_with(BLOCK_E1_NEWLINE) {
            sb.push(BLOCK_E1_NEWLINE);
        }
        sb.push_str(end);
    }
    let display_string = sb;
    let id_short = group(2).unwrap_or(&display_string);
    let partition = group(4).map(unquoted);
    if let Some(partition) = partition {
        let cuca = &mut diagram.cuca;
        let quark = cuca.quark_in_context(true, ActivityDiagram::clean_id(partition))?;
        cuca.goto_group(
            Some(location),
            quark,
            Display::with_newlines(partition),
            GroupType::Package,
        );
    }
    let quark = diagram
        .cuca
        .quark_in_context(true, ActivityDiagram::clean_id(id_short))?;
    let entity2 = diagram.leaf_named(
        location,
        quark,
        Display::with_newlines(&display_string),
        LeafType::Activity,
    );
    diagram.last_entity_consulted = Some(entity2);
    if partition.is_some() {
        diagram.cuca.end_group();
    }
    if url_activity.is_some() {
        diagram.cuca.entity_mut(entity2).url = url_activity;
    }
    set_stereotype_and_color(diagram, entity2, group(3), group(5))?;
    let length = arrow_length(
        line0.get("ARROW_BODY1", 0),
        line0.get("ARROW_DIRECTION", 0),
        line0.get("ARROW_BODY2", 0),
    );
    let label = line0.get("BRACKET", 0).map(Display::with_newlines);
    add_activity_link(
        diagram, location, &line0, entity1, entity2, label, length, false,
    );
    Ok(())
}

fn note_activity_head() -> Vec<RegexTree> {
    vec![
        RegexTree::start(),
        RegexTree::leaf(r"note"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "POSITION", r"(right|left|top|bottom)"),
        RegexTree::spaces_zero_or_more(),
        color::optional_pattern("COLOR"),
        RegexTree::spaces_zero_or_more(),
    ]
}

/// `note left: text` on the latest action (PlantUML's `CommandFactoryNoteActivity.createSingleLine`).
fn note_activity() -> Box<dyn Command<ActivityDiagram>> {
    let mut pattern = note_activity_head();
    pattern.extend([
        RegexTree::leaf(r":"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "NOTE", r"(.*)"),
        RegexTree::end(),
    ]);
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(pattern),
        |diagram: &mut ActivityDiagram, location: &LineLocation, arg: &RegexResult| {
            let tmp = diagram.cuca.get_unique_sequence("GN");
            let quark = diagram
                .cuca
                .quark_in_context(true, ActivityDiagram::clean_id(&tmp))?;
            let display = Display::with_newlines(arg.get("NOTE", 0).unwrap_or_default());
            let note = diagram.really_create_leaf(location, quark, display, LeafType::Note);
            add_note_activity(diagram, location, arg, note)
        },
    )))
}

static END_NOTE: LazyLock<Regex> = LazyLock::new(|| plantuml_regex("^[%s]*end[%s]?note$"));

fn note_activity_multi_line_start() -> RegexTree {
    let mut pattern = note_activity_head();
    pattern.push(RegexTree::end());
    RegexTree::concat(pattern)
}

/// `note left` ... `end note` on the latest action (PlantUML's `CommandFactoryNoteActivity.createMultiLine`).
/// A link alone on the first line goes on the note.
fn note_activity_multi_line() -> Box<dyn Command<ActivityDiagram>> {
    let first_line = note_activity_multi_line_start();
    Box::new(Multiline::starting_with_owned(
        note_activity_multi_line_start(),
        &END_NOTE,
        move |diagram: &mut ActivityDiagram, lines: &BlocLines| {
            let first = lines.first().expect("a block has its first line").trimmed();
            let arg = first_line
                .matcher(first.text())
                .expect("the block's first line matched");
            let body = lines.sub_extract(1, 1).without_empty_columns();
            let location = body
                .first()
                .map_or_else(|| first.location(), |line| line.location())
                .clone();
            let mut strings = body.to_display();
            let url = strings.lines().first().and_then(|line| Url::parse(line));
            if url.is_some() {
                strings = Display::create(strings.lines()[1..].iter().cloned());
            }
            let code_string = diagram.cuca.get_unique_sequence("GMN");
            let quark = diagram.cuca.quark_in_context(true, &code_string)?;
            let note = diagram.really_create_leaf(&location, quark, strings, LeafType::Note);
            diagram.cuca.entity_mut(note).url = url;
            add_note_activity(diagram, &location, &arg, note)
        },
    ))
}

/// Colours the note and links it, dashed, to the latest action, or to the start when there is none
/// (`CommandFactoryNoteActivity.executeInternal`).
fn add_note_activity(
    diagram: &mut ActivityDiagram,
    location: &LineLocation,
    arg: &RegexResult,
    note: EntityId,
) -> CommandResult {
    let back = arg.get("COLOR", 0).map(parse_color).transpose()?;
    let entity = diagram.cuca.entity_mut(note);
    entity.colors = entity.colors.with(ColorType::Back, back);
    let activity = match diagram.last_entity_consulted {
        Some(activity) => activity,
        None => diagram.get_start(location)?,
    };
    let cuca = &mut diagram.cuca;
    let position = Position::from_string(arg.get("POSITION", 0).unwrap_or_default())
        .expect("the pattern only matches positions")
        .with_rankdir(cuca.skin().get_rankdir());
    let (entity1, entity2, length) = match position {
        Position::Right => (activity, note, 1),
        Position::Left => (note, activity, 1),
        Position::Bottom => (activity, note, 2),
        Position::Top => (note, activity, 2),
    };
    let link_type = LinkType::new(LinkDecor::None, LinkDecor::None).go_dashed();
    let link = cuca.new_link(
        Some(location),
        entity1,
        entity2,
        link_type,
        LinkArg::no_display(length),
    );
    cuca.add_link(link);
    Ok(())
}
