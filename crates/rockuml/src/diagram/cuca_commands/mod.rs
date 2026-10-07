//! Commands the diagrams of entities and links share (class, description and state diagrams), which
//! PlantUML keeps in its `command`, `classdiagram`, `descdiagram` and `objectdiagram` packages.

pub(super) mod labels;
pub(super) mod note;

use regex::Regex;

use super::cuca::{CucaDiagram, EntityDiagram};
use super::titled::TitledDiagram;
use crate::abel::{Entity, GroupType, LeafType};
use crate::color::{ColorType, Colors};
use crate::command::unported::{self, NotPortedCommands};
use crate::command::{BlocLines, CommandControl, CommandError, Multiline};
use crate::command::{Command, PatternCommand, SingleLine};
use crate::creole::Display;
use crate::decoration::symbol::{USymbol, USymbols};
use crate::klimt::url::Url;
use crate::pattern::{RegexResult, RegexTree, plantuml_regex};
use crate::skin::Rankdir;
use crate::stereo::{Stereotag, Stereotype};
use crate::text::{LineLocation, StringLocated};
use crate::{color, stereo};

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
        |diagram: &mut D, location: &LineLocation, arg: &RegexResult| {
            let code_arg = CucaDiagram::clean_id(arg.get_lazzy("CODE", 0).unwrap_or_default()).to_owned();
            let colors = colors(arg, ColorType::Back)?;
            let code = if code_arg.is_empty() {
                diagram.cuca().get_unique_sequence("##")
            } else {
                code_arg.clone()
            };
            let code = diagram.clean_id(&code).to_owned();
            let cuca = diagram.cuca();
            let ident = cuca.quark_in_context(false, &code);
            let display = if code_arg.is_empty() {
                Display::default()
            } else {
                let display_arg = arg.get_lazzy("DISPLAY", 0).map(CucaDiagram::clean_id);
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
            if let Some(url) = arg.get("URL", 0).and_then(Url::parse) {
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
            let code = diagram
                .clean_id(arg.get("CODE", 0).unwrap_or_default())
                .to_owned();
            let cuca = diagram.cuca();
            let quark = cuca.quark_in_context(true, &code);
            let Some(entity) = cuca.quark(quark).get_data() else {
                return Err(CommandError::new(format!(
                    "{} does not exist",
                    cuca.quark(quark).get_name()
                )));
            };
            cuca.entity_mut(entity).url = arg.get("URL", 0).and_then(Url::parse);
            Ok(())
        },
    )))
}

/// The keywords that declare description elements, the longer of two that start alike first
/// (`CommandCreateElementFull.ALL_TYPES`).
pub(super) const ALL_TYPES: &str = "person|artifact|actor/|actor|folder|card|file|package|rectangle|hexagon|label|node|frame|cloud|action|process|database|queue|stack|storage|agent|usecase/|usecase|component|boundary|control|entity|interface|circle|collections|port|portin|portout";

/// PlantUML's `CommandCreateElementMultilines` of type 0: `node N as "` with a description up to the
/// closing quote.
pub(super) fn create_element_multilines_type0<D: EntityDiagram + 'static>()
-> Box<dyn Command<D>> {
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
pub(super) fn create_element_multilines_type1<D: EntityDiagram + 'static>()
-> Box<dyn Command<D>> {
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
            let line0 = start_pattern
                .matcher(first.text())
                .expect("the first line matched");
            let last = lines.last().expect("a block has a last line").trimmed();
            let line_last = end_pattern
                .captures(last.text())
                .and_then(|captures| captures.get(1))
                .map_or("", |matched| matched.as_str())
                .to_owned();
            let keyword = line0.get("TYPE", 0).unwrap_or_default();
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
                .unwrap_or_else(|| panic!("{keyword} names a symbol"));
                (LeafType::Description, usymbol)
            };
            let mut texts: Vec<String> = lines
                .sub_extract(1, 1)
                .iter()
                .map(|line| line.text().to_owned())
                .collect();
            if let Some(desc_start) = line0.get("DESC", 0).filter(|desc| !desc.is_empty()) {
                texts.insert(0, desc_start.to_owned());
            }
            if !line_last.is_empty() {
                texts.push(line_last);
            }
            let display = Display::create(texts);
            let colors = colors(&line0, ColorType::Back)?;
            let code = diagram
                .clean_id(line0.get("CODE", 0).unwrap_or_default())
                .to_owned();
            let cuca = diagram.cuca();
            let quark = cuca.quark_in_context(true, &code);
            let entity = match cuca.quark(quark).get_data() {
                Some(existing) => existing,
                None => {
                    let created =
                        cuca.really_create_leaf(location.as_ref(), quark, display, leaf_type);
                    cuca.entity_mut(created).usymbol = Some(usymbol);
                    created
                }
            };
            if exists_with_bad_type3(cuca.entity(entity), leaf_type, Some(usymbol)) {
                return Err(CommandError::new(format!(
                    "This element ({}) is already defined",
                    cuca.quark(quark).get_name()
                )));
            }
            let entity = cuca.entity_mut(entity);
            if let Some(stereotype) = line0.get("STEREO", 0) {
                entity.stereotype =
                    Some(Stereotype::with_spot(stereotype).map_err(|_| CommandError::bad_color())?);
            }
            if let Some(url) = line0.get("URL", 0).and_then(Url::parse) {
                entity.url = Some(url);
            }
            entity.colors = colors;
            Ok(())
        })
        .skipping_quote_lines(),
    )
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
