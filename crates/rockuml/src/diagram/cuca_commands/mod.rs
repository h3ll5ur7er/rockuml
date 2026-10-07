//! Commands the diagrams of entities and links share (class, description and state diagrams), which
//! PlantUML keeps in its `command`, `classdiagram`, `descdiagram` and `objectdiagram` packages.

pub(super) mod labels;
pub(super) mod note;

use std::sync::LazyLock;

use regex::Regex;

pub(super) use labels::Labels;

use super::cuca::{CucaDiagram, EntityDiagram};
use super::titled::TitledDiagram;
use crate::abel::{Entity, EntityId, GroupType, LeafType, LinkArg};
use crate::color::{ColorType, Colors, HColor};
use crate::command::unported::{self, NotPortedCommands};
use crate::command::{
    BlocLines, Command, CommandControl, CommandError, Multiline, PatternCommand, SingleLine,
};
use crate::creole::Display;
use crate::cucadiagram::get_linked_entry;
use crate::decoration::symbol::{USymbol, USymbols};
use crate::decoration::{LinkDecor, LinkType};
use crate::java;
use crate::json::JsonValue;
use crate::klimt::url::Url;
use crate::pattern::{RegexResult, RegexTree, plantuml_regex};
use crate::plasma::QuarkId;
use crate::skin::Rankdir;
use crate::stereo::{Stereotag, Stereotype};
use crate::text::{LineLocation, StringLocated, without_quotes_or_brackets};
use crate::{color, stereo};

/// A stereotype, whose spot colour must exist (`Stereotype.build` with the circled character font).
pub(super) fn stereotype(stereo: &str) -> Result<Stereotype, CommandError> {
    Stereotype::with_spot(stereo).map_err(|_| CommandError::bad_color())
}

/// The colours of `COLOR`, then the line colour and style of `##[style]color`.
pub(super) fn colors_with_line(arg: &RegexResult) -> Result<Colors, CommandError> {
    let mut colors = colors(arg, ColorType::Back)?;
    if let Some(line_color) = arg.get("LINECOLOR", 1) {
        let color = HColor::parse(line_color)
            .ok()
            .flatten()
            .ok_or_else(CommandError::bad_color)?;
        colors = colors.with(ColorType::Line, Some(color));
    }
    if let Some(style) = arg.get("LINECOLOR", 0) {
        colors = colors.add_legacy_stroke(style);
    }
    Ok(colors)
}

/// The `[[link]]` of the line.
pub(super) fn url_of(arg: &RegexResult) -> Option<Url> {
    arg.get("URL", 0).and_then(Url::parse)
}

/// The display a declaration names, or else its name, read without creole lists.
pub(super) fn display_or_name(display: Option<&str>, name: &str) -> Display {
    display.map_or_else(|| Display::with_newlines(name), Display::with_newlines)
}

/// PlantUML's `CommandFootboxIgnored`: `hide footbox` means nothing outside sequence diagrams.
pub(super) fn footbox_ignored<D: 'static>() -> Box<dyn Command<D>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::counted(1, r"(hide|show)?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"footbox"),
            RegexTree::end(),
        ]),
        |_: &mut D, _: &LineLocation, _: &RegexResult| Ok(()),
    )))
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

/// PlantUML's `CommandHideShow2`: `hide Foo`, `show <<Stereo>>`, `hide $tag`, `hide @unlinked`.
pub(super) fn hide_show2<D: EntityDiagram + 'static>() -> Box<dyn Command<D>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "COMMAND", r"(hide|hide-class|show|show-class)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "WHAT", r"([^%s]+|\<\<.*\>\>)"),
            RegexTree::end(),
        ]),
        |diagram: &mut D, _: &LineLocation, arg: &RegexResult| {
            let show = arg
                .get("COMMAND", 0)
                .is_some_and(|command| command.starts_with(['s', 'S']));
            let what = java::trim(arg.get("WHAT", 0).unwrap_or_default());
            diagram.cuca().hide_or_show2(what, show);
            Ok(())
        },
    )))
}

/// PlantUML's `CommandRemoveRestore`.
pub(super) fn remove_restore<D: EntityDiagram + 'static>() -> Box<dyn Command<D>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "COMMAND", r"(remove|restore)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "WHAT", r"(.+)"),
            RegexTree::end(),
        ]),
        |diagram: &mut D, _: &LineLocation, arg: &RegexResult| {
            let show = arg
                .get("COMMAND", 0)
                .is_some_and(|command| command.eq_ignore_ascii_case("restore"));
            let what = java::trim(arg.get("WHAT", 0).unwrap_or_default());
            diagram.cuca().remove_or_restore(what, show);
            Ok(())
        },
    )))
}

/// The end of the block of a map, an object or a JSON element.
fn block_end() -> Regex {
    plantuml_regex(r"^[%s]*\}[%s]*$")
}

/// PlantUML's `CommandCreateMap`: `map Name {` and `key => value` lines; `key *-> Entity` links the key to
/// an entity.
pub(super) fn create_map<D: EntityDiagram + 'static>() -> Box<dyn Command<D>> {
    static START: LazyLock<RegexTree> = LazyLock::new(|| {
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
        ])
    });
    Box::new(
        Multiline::starting_with(
            &START,
            &block_end(),
            |diagram: &mut D, lines: &BlocLines| {
                let lines = without_empty_lines(&lines.trimmed());
                let first = lines.first().expect("a block has a first line");
                let header = START
                    .matcher(first.text())
                    .expect("checked when the block was recognised");
                let location = first.location();
                let cuca = diagram.cuca();
                let name = header.get("NAME", 1).unwrap_or_default();
                let quark = cuca.quark_in_context(true, D::clean_id(name))?;
                if cuca.quark(quark).get_data().is_some() {
                    return Err(CommandError::new(format!("Map already exists: {name}")));
                }
                let display = display_or_name(header.get("NAME", 0), name);
                let entity = cuca.really_create_leaf(Some(location), quark, display, LeafType::Map);
                if let Some(stereo) = header.get("STEREO", 0) {
                    cuca.entity_mut(entity).stereotype = Some(stereotype(stereo)?);
                }
                cuca.entity_mut(entity).colors = colors_with_line(&header)?;
                for entry in lines.sub_extract(1, 1).iter() {
                    let entry = entry.text();
                    if !cuca.entity_mut(entity).bodier.add_field_or_method(entry) {
                        return Err(CommandError::new(
                            "Map definition should contains key => value",
                        ));
                    }
                    if let Some(arrow) = get_linked_entry(entry) {
                        let x = entry.find(arrow).expect("the link is in the entry");
                        let key = java::trim(&entry[..x]).to_owned();
                        let dest = java::trim(&entry[x + arrow.len()..]);
                        let ident2 = cuca.quark_in_context(true, dest)?;
                        let Some(entity2) = cuca.quark(ident2).get_data() else {
                            return Err(CommandError::new(format!(
                                "No such entity {}",
                                cuca.quark(ident2).get_name()
                            )));
                        };
                        let link_type = LinkType::new(LinkDecor::Arrow, LinkDecor::None);
                        let length = arrow.chars().count() as i32 - 2;
                        let link = cuca.new_link(
                            Some(location),
                            entity,
                            entity2,
                            link_type,
                            LinkArg::no_display(length),
                        );
                        cuca.set_port_members(link, Some(key), None);
                        cuca.add_link(link);
                    }
                }
                Ok(())
            },
        )
        .skipping_quote_lines(),
    )
}

fn without_empty_lines(lines: &BlocLines) -> BlocLines {
    lines
        .iter()
        .filter(|line| !line.text().is_empty())
        .cloned()
        .fold(BlocLines::default(), BlocLines::add)
}

/// The declaration of a JSON element: `json Name` and how it is shown.
fn json_header() -> Vec<RegexTree> {
    vec![
        RegexTree::start(),
        RegexTree::named(0, "TYPE", r"json"),
        RegexTree::spaces_one_or_more(),
    ]
}

/// A JSON element for `quark`, unless it holds one already.
fn create_json_entity(
    cuca: &mut CucaDiagram,
    location: &LineLocation,
    header: &RegexResult,
    quark: QuarkId,
    display: Display,
) -> Result<Option<EntityId>, CommandError> {
    if cuca.quark(quark).get_data().is_some() {
        return Ok(None);
    }
    let entity = cuca.really_create_leaf(Some(location), quark, display, LeafType::Json);
    if let Some(stereo) = header.get("STEREO", 0) {
        cuca.entity_mut(entity).stereotype = Some(stereotype(stereo)?);
    }
    let back = back_color(header)?;
    let entity_mut = cuca.entity_mut(entity);
    entity_mut.colors = entity_mut.colors.with(ColorType::Back, back);
    Ok(Some(entity))
}

/// The single colour of `COLOR`, which paints a background (`setSpecificColorTOBEREMOVED(ColorType.BACK,
/// ...)`).
pub(super) fn back_color(arg: &RegexResult) -> Result<Option<HColor>, CommandError> {
    arg.get("COLOR", 0)
        .map(|color| {
            HColor::parse(color)
                .ok()
                .flatten()
                .ok_or_else(CommandError::bad_color)
        })
        .transpose()
}

/// PlantUML's `CommandCreateJson`.
pub(super) fn create_json<D: EntityDiagram + 'static>() -> Box<dyn Command<D>> {
    static START: LazyLock<RegexTree> = LazyLock::new(|| {
        let mut parts = json_header();
        parts.extend([
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
        ]);
        RegexTree::concat(parts)
    });
    Box::new(
        Multiline::starting_with(
            &START,
            &block_end(),
            |diagram: &mut D, lines: &BlocLines| {
                let lines = without_empty_lines(&lines.trimmed());
                let first = lines.first().expect("a block has a first line");
                let header = START
                    .matcher(first.text())
                    .expect("checked when the block was recognised");
                let code = header.get_lazzy("CODE", 0).unwrap_or_default();
                let cuca = diagram.cuca();
                let quark = cuca.quark_in_context(true, D::clean_id(code))?;
                let display =
                    display_or_name(header.get_lazzy("DISPLAY", 0), cuca.quark(quark).get_name());
                let Some(entity) =
                    create_json_entity(cuca, first.location(), &header, quark, display)?
                else {
                    return Err(CommandError::new(format!("JSON already exists: {code}")));
                };
                let Some(json) = json_value(&lines) else {
                    return Err(CommandError::new("Bad data"));
                };
                cuca.entity_mut(entity).bodier.set_json(json);
                Ok(())
            },
        )
        .skipping_quote_lines()
        .verified_by(json_is_complete),
    )
}

/// The data of a JSON block, with or without the braces around it.
fn json_value(lines: &BlocLines) -> Option<JsonValue> {
    let inner: String = lines
        .sub_extract(1, 1)
        .iter()
        .map(StringLocated::text)
        .collect();
    crate::json::parse(&format!("{{{inner}}}"))
        .or_else(|_| crate::json::parse(&inner))
        .ok()
}

/// PlantUML's `CommandCreateJsonSingleLine`.
pub(super) fn create_json_single_line<D: EntityDiagram + 'static>() -> Box<dyn Command<D>> {
    let mut parts = json_header();
    parts.extend([
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
    ]);
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(parts),
        |diagram: &mut D, location: &LineLocation, arg: &RegexResult| {
            let name = arg.get("NAME", 1).unwrap_or_default();
            let cuca = diagram.cuca();
            let quark = cuca.quark_in_context(false, D::clean_id(name))?;
            let display = display_or_name(arg.get("NAME", 0), name);
            let Some(entity) = create_json_entity(cuca, location, arg, quark, display)? else {
                return Err(CommandError::new(format!("JSON already exists: {name}")));
            };
            let Ok(json) = crate::json::parse(arg.get_lazzy("DATA_", 0).unwrap_or_default()) else {
                return Err(CommandError::new("Bad data"));
            };
            diagram.cuca().entity_mut(entity).bodier.set_json(json);
            Ok(())
        },
    )))
}

/// PlantUML's `CommandEndPackage`: `}` leaves the innermost group or `together` block.
pub(super) fn end_package<D: EntityDiagram + 'static>() -> Box<dyn Command<D>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"\}"),
            RegexTree::end(),
        ]),
        |diagram: &mut D, _: &LineLocation, _: &RegexResult| {
            if diagram.cuca().end_group() {
                Ok(())
            } else {
                Err(CommandError::new("No package or namespace defined"))
            }
        },
    )))
}

/// PlantUML's `CommandPackageWithUSymbol`: `node "Name" as N {` opens a group drawn as the symbol.
pub(super) fn package_with_usymbol<D: EntityDiagram + 'static>() -> Box<dyn Command<D>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(
                1,
                "SYMBOL",
                r"(package|rectangle|hexagon|node|artifact|folder|file|frame|cloud|action|process|database|storage|component|card|queue|stack)",
            ),
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
        |diagram: &mut D, location: &LineLocation, arg: &RegexResult| {
            let code_arg =
                without_quotes_or_brackets(arg.get_lazzy("CODE", 0).unwrap_or_default()).to_owned();
            let colors = colors(arg, ColorType::Back)?;
            let code = if code_arg.is_empty() {
                diagram.cuca().get_unique_sequence("##")
            } else {
                code_arg.clone()
            };
            let code = D::clean_id(&code).to_owned();
            let cuca = diagram.cuca();
            let ident = cuca.quark_in_context(false, &code)?;
            let display = if code_arg.is_empty() {
                Display::default()
            } else {
                let display_arg = arg.get_lazzy("DISPLAY", 0).map(without_quotes_or_brackets);
                Display::with_newlines(display_arg.unwrap_or(cuca.quark(ident).get_name()))
            };
            let skin = cuca.skin();
            let usymbol = USymbols::from_string(
                arg.get("SYMBOL", 0).unwrap_or_default(),
                skin.actor_style(),
                skin.component_style(),
                skin.package_style(),
            );
            cuca.goto_group(Some(location), ident, display, GroupType::Package);
            let group = cuca.get_current_group();
            let entity = cuca.entity_mut(group);
            if usymbol.is_some() {
                entity.usymbol = usymbol;
            }
            if let Some(stereotype) = arg.get_lazzy("STEREOTYPE", 0) {
                entity.stereotype = Some(Stereotype::new(stereotype));
            }
            if let Some(url) = url_of(arg) {
                entity.url = Some(url);
            }
            add_tags(entity, arg.get_lazzy("TAGS", 0));
            entity.colors = colors;
            Ok(())
        },
    )))
}

/// PlantUML's `CommandTogether`: `together {` keeps the elements up to its `}` close in the layout.
pub(super) fn together<D: EntityDiagram + 'static>() -> Box<dyn Command<D>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"together"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{"),
            RegexTree::end(),
        ]),
        |diagram: &mut D, _: &LineLocation, _: &RegexResult| {
            diagram.cuca().goto_together();
            Ok(())
        },
    )))
}

/// PlantUML's `CommandUrl`: `url of A is [[...]]`.
pub(super) fn url<D: EntityDiagram + 'static>() -> Box<dyn Command<D>> {
    Box::new(SingleLine(PatternCommand::new(
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
        |diagram: &mut D, _: &LineLocation, arg: &RegexResult| {
            let code = without_quotes_or_brackets(arg.get("CODE", 0).unwrap_or_default());
            let cuca = diagram.cuca();
            let quark = cuca.quark_in_context(true, D::clean_id(code))?;
            let Some(entity) = cuca.quark(quark).get_data() else {
                return Err(CommandError::new(format!(
                    "{} does not exist",
                    cuca.quark(quark).get_name()
                )));
            };
            cuca.entity_mut(entity).url = url_of(arg);
            Ok(())
        },
    )))
}

/// The keywords that declare description elements, the longer of two that start alike first
/// (`CommandCreateElementFull.ALL_TYPES`).
pub(super) const ALL_TYPES: &str = "person|artifact|actor/|actor|folder|card|file|package|rectangle|hexagon|label|node|frame|cloud|action|process|database|queue|stack|storage|agent|usecase/|usecase|component|boundary|control|entity|interface|circle|collections|port|portin|portout";

/// PlantUML's `CommandCreateElementMultilines` of type 0: `node N as "` with a description up to the
/// closing quote.
pub(super) fn create_element_multilines_type0<D: EntityDiagram + 'static>() -> Box<dyn Command<D>> {
    fn start() -> RegexTree {
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "TYPE", format!("({ALL_TYPES})[%s]+")),
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
        ])
    }
    create_element_multilines(start, &plantuml_regex(r"^(.*)[%g]$"))
}

/// PlantUML's `CommandCreateElementMultilines` of type 1: `node N [` with a description up to `]`.
pub(super) fn create_element_multilines_type1<D: EntityDiagram + 'static>() -> Box<dyn Command<D>> {
    fn start() -> RegexTree {
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "TYPE", format!("({ALL_TYPES})[%s]+")),
            RegexTree::named(1, "CODE", r"([%pLN_.]+)"),
            stereo::optional_pattern("STEREO"),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\["),
            RegexTree::named(1, "DESC", r"(.*)"),
            RegexTree::end(),
        ])
    }
    create_element_multilines(start, &plantuml_regex(r"^([^\[\]]*)\]$"))
}

fn create_element_multilines<D: EntityDiagram + 'static>(
    start: fn() -> RegexTree,
    end: &Regex,
) -> Box<dyn Command<D>> {
    let end_pattern = Regex::new(&format!("^(?:{})$", end.as_str())).expect("a valid pattern");
    let start_pattern = start();
    Box::new(
        Multiline::starting_with_owned(start(), end, move |diagram: &mut D, lines: &BlocLines| {
            let location = lines.first().map(|first| first.location().clone());
            let lines = lines.trim_smart(1);
            let first = lines.first().expect("a block has a first line").trimmed();
            let head = start_pattern
                .matcher(first.text())
                .expect("the first line matched");
            let last = lines.last().expect("a block has a last line").trimmed();
            let line_last = end_pattern
                .captures(last.text())
                .and_then(|captures| captures.get(1))
                .map_or("", |matched| matched.as_str())
                .to_owned();
            let keyword = head.get("TYPE", 0).unwrap_or_default();
            let (leaf_type, usymbol) = if keyword.eq_ignore_ascii_case("usecase") {
                (LeafType::Usecase, USymbols::USECASE)
            } else if keyword.eq_ignore_ascii_case("usecase/") {
                (LeafType::UsecaseBusiness, USymbols::USECASE_BUSINESS)
            } else {
                let skin = diagram.cuca().skin();
                let usymbol = USymbols::from_string(
                    keyword,
                    skin.actor_style(),
                    skin.component_style(),
                    skin.package_style(),
                )
                .ok_or_else(|| unknown_symbol(keyword))?;
                (LeafType::Description, usymbol)
            };
            let mut texts: Vec<String> = lines
                .sub_extract(1, 1)
                .iter()
                .map(|line| line.text().to_owned())
                .collect();
            if let Some(desc_start) = head.get("DESC", 0).filter(|desc| !desc.is_empty()) {
                texts.insert(0, desc_start.to_owned());
            }
            if !line_last.is_empty() {
                texts.push(line_last);
            }
            let display = Display::create(texts);
            let colors = colors(&head, ColorType::Back)?;
            let code = D::clean_id(head.get("CODE", 0).unwrap_or_default());
            let cuca = diagram.cuca();
            let quark = cuca.quark_in_context(true, code)?;
            let entity = if let Some(existing) = cuca.quark(quark).get_data() {
                existing
            } else {
                let created = cuca.really_create_leaf(location.as_ref(), quark, display, leaf_type);
                cuca.entity_mut(created).usymbol = Some(usymbol);
                created
            };
            if exists_with_bad_type3(cuca.entity(entity), leaf_type, Some(usymbol)) {
                return Err(CommandError::new(format!(
                    "This element ({}) is already defined",
                    cuca.quark(quark).get_name()
                )));
            }
            let entity = cuca.entity_mut(entity);
            if let Some(stereotype) = head.get("STEREO", 0) {
                entity.stereotype =
                    Some(Stereotype::with_spot(stereotype).map_err(|_| CommandError::bad_color())?);
            }
            if let Some(url) = head.get("URL", 0).and_then(Url::parse) {
                entity.url = Some(url);
            }
            entity.colors = colors;
            Ok(())
        })
        .skipping_quote_lines(),
    )
}

/// A keyword the declaration patterns accept but no symbol answers to, on which PlantUML fails.
pub(super) fn unknown_symbol(keyword: &str) -> CommandError {
    CommandError::new(format!("No symbol is named {keyword}"))
}

/// Whether `other` cannot be declared again as `leaf_type` drawn as `usymbol`
/// (`CommandCreateElementFull.existsWithBadType3`).
pub(super) fn exists_with_bad_type3(
    other: &Entity,
    leaf_type: LeafType,
    usymbol: Option<USymbol>,
) -> bool {
    other.get_leaf_type() != Some(leaf_type)
        || usymbol.is_some_and(|usymbol| other.get_usymbol() != Some(usymbol))
}

/// `$tag1 $tag2`, which `hide $tag1` selects (`CommandCreateClassMultilines.addTags`).
pub(super) fn add_tags(entity: &mut Entity, tags: Option<&str>) {
    let Some(tags) = tags else {
        return;
    };
    for tag in tags.split(' ').filter(|tag| !tag.is_empty()) {
        entity.add_stereotag(Stereotag {
            name: tag.strip_prefix('$').unwrap_or(tag).to_owned(),
        });
    }
}

/// The colours a `COLOR` specification gives, the main one painting `main_type`.
pub(super) fn colors(arg: &RegexResult, main_type: ColorType) -> Result<Colors, CommandError> {
    arg.get("COLOR", 0)
        .map(|data| Colors::parse(data, main_type).map_err(|_| CommandError::bad_color()))
        .transpose()
        .map(Option::unwrap_or_default)
}

/// Whether the block holds JSON data, with or without the braces around it
/// (`CommandCreateJson.finalVerification`); until it does, more lines may complete it.
fn json_is_complete(lines: &BlocLines) -> CommandControl {
    if json_value(lines).is_some() {
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
