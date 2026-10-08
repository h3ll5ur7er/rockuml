//! The commands of class and object diagrams (PlantUML's `classdiagram.command` and `objectdiagram.command`
//! packages, and the package commands they borrow).

use std::sync::LazyLock;

use regex::Regex;

use super::ClassDiagram;
use crate::abel::{EntityId, GroupType, LeafType, Link, LinkArg};
use crate::color::ColorType;
use crate::command::unported::{self, NotPortedCommands};
use crate::command::{
    BlocLines, Command, CommandError, CommandResult, Multiline, PatternCommand, SingleLine,
    SingleLineCommand,
};
use crate::creole::Display;
use crate::decoration::symbol::USymbols;
use crate::decoration::{LinkDecor, LinkType, WithLinkType};
use crate::diagram::cuca::{CucaDiagram, EntityDiagram};
use crate::diagram::cuca_commands::{
    GENERIC, Labels, add_tags, back_color, char_encoding, colors, colors_with_line,
    display_or_name, display_with_generic, is_bare_name, unknown_symbol, url_of,
};
use crate::diagram::description::arrow_style;
use crate::direction::Direction;
use crate::java;
use crate::klimt::url::Url;
use crate::pattern::{RegexResult, RegexTree, plantuml_regex};
use crate::plasma::QuarkId;
use crate::skin::visibility_modifier::VisibilityModifier;
use crate::stereo::Stereotype;
use crate::text::{LineLocation, unquoted, without_quotes_or_brackets};
use crate::{color, stereo};

/// Names separated by `.`, `::`, `\\` or other punctuation (`CommandLinkClass.getSeparator`).
const CODES: &str = r"(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_$]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_$]+)*(?:\s*,\s*(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_$]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_$]+)*)*";

/// PlantUML's `CommandAddMethod`: `Foo : member`.
pub(super) fn add_method() -> Box<dyn Command<ClassDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "NAME", r"([%pLN_.]+|[%g][^%g]+[%g])"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r":"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "DATA", r"(.*)"),
            RegexTree::end(),
        ]),
        |diagram: &mut ClassDiagram, location: &LineLocation, arg: &RegexResult| {
            let name = without_quotes_or_brackets(arg.get("NAME", 0).unwrap_or_default());
            let cuca = diagram.cuca();
            let quark = cuca.quark_in_context(true, ClassDiagram::clean_id(name))?;
            let entity = get_or_create_class(cuca, location, quark);
            let field = arg.get("DATA", 0).unwrap_or_default();
            cuca.entity_mut(entity).bodier.add_field_or_method(field)?;
            Ok(())
        },
    )))
}

/// What declares a class: its type, visibility, name, display and generic.
fn class_declaration(types: &'static str) -> Vec<RegexTree> {
    vec![
        RegexTree::start(),
        RegexTree::named(
            1,
            "VISIBILITY",
            format!("({})?", VisibilityModifier::REGEX_FOR_VISIBILITY_CHARACTER),
        ),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "TYPE", types),
        RegexTree::spaces_one_or_more(),
        name_and_code_for_class_with_generic(),
        RegexTree::optional(RegexTree::concat(vec![
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "GENERIC", format!(r"\<({GENERIC})\>")),
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
    ]
}

/// `"Display<Generic>" as Code`, `Code as "Display"`, `Code` or `"Code"`
/// (`NameAndCodeParser.nameAndCodeForClassWithGeneric`).
fn name_and_code_for_class_with_generic() -> RegexTree {
    let display = display_with_generic();
    RegexTree::or(vec![
        RegexTree::concat(vec![
            RegexTree::named(2, "DISPLAY1", display.clone()),
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
            RegexTree::named(2, "DISPLAY2", display),
        ]),
        RegexTree::named(1, "CODE3", r"([^%s{}%g<>]+)"),
        RegexTree::named(1, "CODE4", r"[%g]([^%g]+)[%g]"),
    ])
}

/// `extends A, B` or `implements C`, optionally with generics in the multi-line form.
fn extends_or_implements(name: &'static str, keyword: &'static str, generic: bool) -> RegexTree {
    let mut parts = vec![
        RegexTree::spaces_one_or_more(),
        RegexTree::named(
            3,
            name,
            format!(r"({keyword})[%s]+({CODES}|[%g]([^%g]+)[%g])"),
        ),
    ];
    if generic {
        parts.push(RegexTree::optional(RegexTree::concat(vec![
            RegexTree::spaces_zero_or_more(),
            RegexTree::counted(1, format!(r"\<({GENERIC})\>")),
        ])));
    }
    RegexTree::optional(RegexTree::concat(parts))
}

/// How a class declaration failed to name a new or compatible entity.
struct Refused {
    quark_failure: fn(CommandError) -> CommandError,
    incompatible: fn(&str) -> CommandError,
}

/// Declares the class `header` names, or changes the entity already named so (`CommandCreateClass` and
/// `CommandCreateClassMultilines`, which differ in their errors and in keeping an existing display).
fn create_class_entity(
    diagram: &mut ClassDiagram,
    location: &LineLocation,
    header: &RegexResult,
    refused: &Refused,
    replace_display: bool,
) -> Result<EntityId, CommandError> {
    let type_string = header.get("TYPE", 0).unwrap_or_default().to_uppercase();
    let leaf_type =
        LeafType::get_leaf_type(&type_string).expect("the pattern only accepts type keywords");
    let visibility = header.get("VISIBILITY", 0).and_then(|visibility| {
        VisibilityModifier::get_visibility_modifier(&format!("{visibility}FOO"), false)
    });
    let id_short = ClassDiagram::clean_id(header.get_lazzy("CODE", 0).unwrap_or_default());
    let display_string = header.get_lazzy("DISPLAY", 0);
    let generic = header
        .get_lazzy("DISPLAY", 1)
        .or_else(|| header.get("GENERIC", 0));
    let cuca = diagram.cuca();
    let quark = cuca
        .quark_in_context(false, id_short)
        .map_err(refused.quark_failure)?;
    let entity = match cuca.quark(quark).get_data() {
        None => {
            let display = display_or_name(display_string, cuca.quark(quark).get_name());
            cuca.really_create_leaf(Some(location), quark, display, leaf_type)
        }
        Some(entity) => {
            if !cuca
                .entity_mut(entity)
                .mute_to_type_if_compatible(leaf_type)
            {
                return Err((refused.incompatible)(id_short));
            }
            if replace_display && let Some(display) = display_string {
                cuca.entity_mut(entity).display = Display::with_newlines(display);
            }
            entity
        }
    };
    check_if_package_hierarchy_is_ok(cuca, quark)?;
    cuca.set_last_entity(Some(entity));
    let stereo = header.get("STEREO", 0);
    let stereotype = stereo.map(Stereotype::with_spot).transpose()?;
    let colors = colors_with_line(header)?;
    let entity_mut = cuca.entity_mut(entity);
    entity_mut.visibility_modifier = visibility;
    if stereotype.is_some() {
        entity_mut.stereotype = stereotype;
        entity_mut.stereostyles = crate::stereo::stereostyles(stereo.unwrap_or_default());
    }
    if let Some(url) = url_of(header) {
        entity_mut.url = Some(url);
    }
    entity_mut.colors = colors;
    if let Some(generic) = generic {
        entity_mut.generic = Some(generic.to_owned());
    }
    if type_string.contains("STATIC") {
        entity_mut.is_static = true;
    }
    Ok(entity)
}

/// A class may only be inside packages (`ClassDiagram.checkIfPackageHierarchyIsOk`).
fn check_if_package_hierarchy_is_ok(cuca: &CucaDiagram, quark: QuarkId) -> CommandResult {
    let mut current = cuca.quark(quark).get_parent();
    while let Some(parent) = current.filter(|parent| !parent.is_root()) {
        if let Some(data) = cuca.quark(parent).get_data()
            && !cuca.entity(data).is_group()
        {
            return Err(CommandError::new(format!(
                "Bad hierarchy for class {}",
                cuca.quark(quark).get_qualified_name()
            )));
        }
        current = cuca.quark(parent).get_parent();
    }
    Ok(())
}

/// The classes after `extends` or `implements` get an arrow to `entity`, created as classes or interfaces
/// as needed (`CommandCreateClassMultilines.manageExtends`).
fn manage_extends(
    location: &LineLocation,
    keyword: &str,
    diagram: &mut ClassDiagram,
    arg: &RegexResult,
    entity: EntityId,
) -> CommandResult {
    let (Some(mode), Some(codes)) = (arg.get(keyword, 0), arg.get(keyword, 1)) else {
        return Ok(());
    };
    let extends = mode.eq_ignore_ascii_case("extends");
    let cuca = diagram.cuca();
    let entity_type = cuca.entity(entity).get_leaf_type();
    let type2 = if !extends || entity_type == Some(LeafType::Interface) {
        LeafType::Interface
    } else {
        LeafType::Class
    };
    for code in java::split(without_quotes_or_brackets(codes), ",") {
        let id_short = java::trim(&code);
        let quark = cuca.quark_in_context(false, ClassDiagram::clean_id(id_short))?;
        let cl2 = get_or_create(cuca, location, quark, type2);
        let mut link_type = LinkType::new(LinkDecor::None, LinkDecor::Extends);
        if type2 == LeafType::Interface && entity_type != Some(LeafType::Interface) {
            link_type = link_type.go_dashed();
        }
        let link = cuca.new_link(
            Some(location),
            cl2,
            entity,
            link_type,
            LinkArg::no_display(2),
        );
        cuca.add_link(link);
    }
    Ok(())
}

/// PlantUML's `CommandCreateClassMultilines`: `class Foo {`, its members, `}`.
pub(super) fn create_class_multilines() -> Box<dyn Command<ClassDiagram>> {
    static START: LazyLock<RegexTree> = LazyLock::new(|| {
        let mut parts = class_declaration(
            r"(interface|enum|annotation|abstract[%s]+class|static[%s]+class|abstract|class|entity|protocol|struct|exception|metaclass|stereotype|dataclass|record)",
        );
        parts.extend([
            extends_or_implements("EXTENDS", "extends", true),
            extends_or_implements("IMPLEMENTS", "implements", true),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]);
        RegexTree::concat(parts)
    });
    static END: LazyLock<Regex> = LazyLock::new(|| plantuml_regex(r"^[%s]*\}[%s]*$"));
    Box::new(
        Multiline::starting_with(
            &START,
            &END,
            |diagram: &mut ClassDiagram, lines: &BlocLines| {
                let lines = lines.trim_smart(1);
                let first = lines.first().expect("a block has a first line");
                let header = START
                    .matcher(first.trimmed().text())
                    .expect("checked when the block was recognised");
                let location = first.location();
                let entity = create_class_entity(
                    diagram,
                    location,
                    &header,
                    &Refused {
                        quark_failure: |error| CommandError::new(error.message),
                        incompatible: |id| {
                            CommandError::new(format!(
                                "Cannot create {id} because it already exists"
                            ))
                        },
                    },
                    true,
                )?;
                if lines.len() > 1 {
                    let bodier = &mut diagram.cuca().entity_mut(entity).bodier;
                    for line in lines.sub_extract(1, 1).iter() {
                        bodier.add_field_or_method(line.text())?;
                    }
                }
                manage_extends(location, "EXTENDS", diagram, &header, entity)?;
                manage_extends(location, "IMPLEMENTS", diagram, &header, entity)?;
                add_tags(
                    diagram.cuca().entity_mut(entity),
                    header.get_lazzy("TAGS", 0),
                );
                Ok(())
            },
        )
        .skipping_quote_lines()
        .with_final_bracket(),
    )
}

/// PlantUML's `CommandCreateClass`: `class Foo` on one line.
pub(super) fn create_class() -> Box<dyn Command<ClassDiagram>> {
    let mut parts = class_declaration(
        r"(interface|enum|annotation|abstract[%s]+class|static[%s]+class|abstract|class|entity|circle|diamond|protocol|struct|exception|metaclass|stereotype|dataclass|record|map)",
    );
    parts.extend([
        extends_or_implements("EXTENDS", "extends", false),
        extends_or_implements("IMPLEMENTS", "implements", false),
        RegexTree::optional(RegexTree::concat(vec![
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\}"),
        ])),
        RegexTree::end(),
    ]);
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(parts),
        |diagram: &mut ClassDiagram, location: &LineLocation, arg: &RegexResult| {
            let entity = create_class_entity(
                diagram,
                location,
                arg,
                &Refused {
                    quark_failure: |error| error,
                    incompatible: |_| CommandError::new("Bad name"),
                },
                false,
            )?;
            manage_extends(location, "EXTENDS", diagram, arg, entity)?;
            manage_extends(location, "IMPLEMENTS", diagram, arg, entity)?;
            add_tags(diagram.cuca().entity_mut(entity), arg.get_lazzy("TAGS", 0));
            Ok(())
        },
    )))
}

/// `object "Display" as code`, `object code as "Display"`, `object code` or `object "code"`.
fn object_declaration() -> Vec<RegexTree> {
    vec![
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
    ]
}

/// The stereotype and colour of an object's declaration.
fn decorate_object(
    cuca: &mut CucaDiagram,
    entity: EntityId,
    header: &RegexResult,
) -> CommandResult {
    if let Some(stereo) = header.get("STEREO", 0) {
        cuca.entity_mut(entity).stereotype = Some(Stereotype::with_spot(stereo)?);
    }
    let back = back_color(header)?;
    let entity = cuca.entity_mut(entity);
    entity.colors = entity.colors.with(ColorType::Back, back);
    Ok(())
}

/// PlantUML's `CommandCreateEntityObjectMultilines`: `object foo {`, its fields, `}`.
pub(super) fn create_entity_object_multilines() -> Box<dyn Command<ClassDiagram>> {
    static START: LazyLock<RegexTree> = LazyLock::new(|| {
        let mut parts = object_declaration();
        parts.extend([
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{"),
            RegexTree::end(),
        ]);
        RegexTree::concat(parts)
    });
    static END: LazyLock<Regex> = LazyLock::new(|| plantuml_regex(r"^[%s]*\}[%s]*$"));
    Box::new(
        Multiline::starting_with(
            &START,
            &END,
            |diagram: &mut ClassDiagram, lines: &BlocLines| {
                let lines: Vec<_> = lines
                    .trimmed()
                    .iter()
                    .filter(|line| !line.text().is_empty())
                    .cloned()
                    .collect();
                let header = START
                    .matcher(lines[0].text())
                    .expect("checked when the block was recognised");
                let cuca = diagram.cuca();
                let id_short =
                    ClassDiagram::clean_id(header.get_lazzy("CODE", 0).unwrap_or_default());
                let quark = cuca.quark_in_context(true, id_short)?;
                let entity = if let Some(entity) = cuca.quark(quark).get_data() {
                    entity
                } else {
                    let display = display_or_name(
                        header.get_lazzy("DISPLAY", 0),
                        cuca.quark(quark).get_name(),
                    );
                    cuca.really_create_leaf(
                        Some(lines[0].location()),
                        quark,
                        display,
                        LeafType::Object,
                    )
                };
                decorate_object(cuca, entity, &header)?;
                let bodier = &mut cuca.entity_mut(entity).bodier;
                for line in &lines[1..lines.len() - 1] {
                    bodier.add_field_or_method(line.text())?;
                }
                Ok(())
            },
        )
        .skipping_quote_lines(),
    )
}

/// PlantUML's `CommandCreateEntityObject`: `object foo`.
pub(super) fn create_entity_object() -> Box<dyn Command<ClassDiagram>> {
    let mut parts = object_declaration();
    parts.push(RegexTree::end());
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(parts),
        |diagram: &mut ClassDiagram, location: &LineLocation, arg: &RegexResult| {
            let cuca = diagram.cuca();
            let id_short = ClassDiagram::clean_id(arg.get_lazzy("CODE", 0).unwrap_or_default());
            let quark = cuca.quark_in_context(true, ClassDiagram::clean_id(id_short))?;
            if cuca.quark(quark).get_data().is_some() {
                return Err(CommandError::new(format!(
                    "Object already exists: {}",
                    cuca.quark(quark).get_name()
                )));
            }
            let display =
                display_or_name(arg.get_lazzy("DISPLAY", 0), cuca.quark(quark).get_name());
            let entity = cuca.really_create_leaf(Some(location), quark, display, LeafType::Object);
            decorate_object(cuca, entity, arg)?;
            if let Some(url) = url_of(arg) {
                cuca.entity_mut(entity).url = Some(url);
            }
            Ok(())
        },
    )))
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
    .forbidding(is_bare_name)
    .boxed()
}

/// PlantUML's `CommandLayoutNewLine`, which counts rows that nothing lays out by.
pub(super) fn layout_new_line() -> Box<dyn Command<ClassDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"layout_new_line"),
            RegexTree::end(),
        ]),
        |_: &mut ClassDiagram, _: &LineLocation, _: &RegexResult| Ok(()),
    )))
}

/// PlantUML's `CommandPackage`: `package name {` or `package "Display" as name {`.
pub(super) fn package() -> Box<dyn Command<ClassDiagram>> {
    Box::new(SingleLine(Package(RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::named(
            1,
            "VISIBILITY",
            format!("({})?", VisibilityModifier::REGEX_FOR_VISIBILITY_CHARACTER),
        ),
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
    ]))))
}

struct Package(RegexTree);

impl SingleLineCommand<ClassDiagram> for Package {
    fn pattern(&self) -> &RegexTree {
        &self.0
    }

    fn syntax_with_final_bracket(&self) -> bool {
        true
    }

    fn execute_arg(
        &self,
        diagram: &mut ClassDiagram,
        location: &LineLocation,
        arg: &RegexResult,
    ) -> CommandResult {
        let cuca = diagram.cuca();
        let name = without_quotes_or_brackets(arg.get("NAME", 0).unwrap_or_default());
        let (quark, display) = match arg.get("AS", 0) {
            None if name.is_empty() => return Err(CommandError::new("Error in name")),
            None => {
                let quark = cuca.quark_in_context(false, ClassDiagram::clean_id(name))?;
                (quark, cuca.quark(quark).get_name().to_owned())
            }
            Some(code) => (
                cuca.quark_in_context(false, ClassDiagram::clean_id(code))?,
                name.to_owned(),
            ),
        };
        let stereotype = arg.get("STEREOTYPE", 0);
        let skin = cuca.skin();
        let usymbol = stereotype.and_then(|stereotype| {
            USymbols::from_string(
                stereotype,
                skin.actor_style(),
                skin.component_style(),
                skin.package_style(),
            )
        });
        cuca.goto_group(
            Some(location),
            quark,
            Display::with_newlines(&display),
            GroupType::Package,
        );
        let colors = colors(arg, ColorType::Back)?;
        let group = cuca.get_current_group();
        let entity = cuca.entity_mut(group);
        if usymbol.is_some() {
            entity.usymbol = usymbol;
        }
        if let Some(visibility) = arg.get("VISIBILITY", 0) {
            entity.visibility_modifier =
                VisibilityModifier::get_visibility_modifier(&format!("{visibility}FOO"), false);
        }
        if let Some(stereotype) = stereotype.filter(|_| usymbol.is_none()) {
            entity.stereotype = Some(Stereotype::new(stereotype));
        }
        add_tags(entity, arg.get_lazzy("TAGS", 0));
        if let Some(url) = url_of(arg) {
            entity.url = Some(url);
        }
        entity.colors = colors;
        Ok(())
    }
}

/// PlantUML's `CommandPackageEmpty`: `package name {}`.
pub(super) fn package_empty() -> Box<dyn Command<ClassDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
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
        |diagram: &mut ClassDiagram, location: &LineLocation, arg: &RegexResult| {
            let cuca = diagram.cuca();
            let display = without_quotes_or_brackets(arg.get("DISPLAY", 0).unwrap_or_default());
            let (id_short, display) = match arg.get("CODE", 0) {
                None if display.is_empty() => (cuca.get_unique_sequence("##"), Display::default()),
                None => (display.to_owned(), Display::with_newlines(display)),
                Some(code) => (code.to_owned(), Display::with_newlines(display)),
            };
            let quark = cuca.quark_in_context(false, ClassDiagram::clean_id(&id_short))?;
            cuca.goto_group(Some(location), quark, display, GroupType::Package);
            let back = back_color(arg)?;
            let group = cuca.get_current_group();
            let entity = cuca.entity_mut(group);
            entity.colors = entity.colors.with(ColorType::Back, back);
            cuca.end_group();
            Ok(())
        },
    )))
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
        is_bare_name(line)
    }

    fn execute_arg(
        &self,
        diagram: &mut ClassDiagram,
        location: &LineLocation,
        arg: &RegexResult,
    ) -> CommandResult {
        if self.mode == Mode::NormalKeyword && !diagram.allow_mixing {
            return Err(CommandError::new(
                "Use 'allowmixing' if you want to mix classes and other UML elements.",
            ));
        }
        let mut code_raw = arg.get_lazzy("CODE", 0).unwrap_or_default().to_owned();
        let display_raw = arg.get_lazzy("DISPLAY", 0).map(without_quotes_or_brackets);
        let code_char = char_encoding(Some(&code_raw));
        let code_display = char_encoding(display_raw);
        let symbol = if let Some(rest) = code_raw.strip_prefix("()") {
            code_raw = without_quotes_or_brackets(java::trim(rest)).to_owned();
            "interface"
        } else if code_char == Some('(') || code_display == Some('(') {
            "usecase"
        } else if code_char == Some(':') || code_display == Some(':') {
            "actor"
        } else if code_char == Some('[') || code_display == Some('[') {
            "component"
        } else {
            arg.get("SYMBOL", 0).unwrap_or_default()
        };
        let cuca = diagram.cuca();
        let (leaf_type, usymbol) = match symbol.to_lowercase().as_str() {
            "port" | "portin" => (LeafType::Portin, None),
            "portout" => (LeafType::Portout, None),
            "usecase" => (LeafType::Usecase, None),
            "usecase/" => (LeafType::UsecaseBusiness, None),
            "state" => (LeafType::State, None),
            _ => (
                LeafType::Description,
                Some(
                    USymbols::from_string_skin_param(symbol, cuca.skin())
                        .ok_or_else(|| unknown_symbol(symbol))?,
                ),
            ),
        };
        let id_short = without_quotes_or_brackets(&code_raw).to_owned();
        let display = Display::with_newlines(display_raw.unwrap_or(&id_short));
        let quark = cuca.quark_in_context(true, &id_short)?;
        let entity = if let Some(entity) = cuca.quark(quark).get_data() {
            entity
        } else {
            let entity = cuca.really_create_leaf(Some(location), quark, display.clone(), leaf_type);
            cuca.entity_mut(entity).usymbol = usymbol;
            entity
        };
        let stereotype = arg
            .get_lazzy("STEREOTYPE", 0)
            .map(Stereotype::with_spot)
            .transpose()?;
        let back = back_color(arg)?;
        let entity = cuca.entity_mut(entity);
        entity.display = display;
        if stereotype.is_some() {
            entity.stereotype = stereotype;
        }
        add_tags(entity, arg.get_lazzy("TAGS", 0));
        if let Some(url) = url_of(arg) {
            entity.url = Some(url);
        }
        entity.colors = entity.colors.with(ColorType::Back, back);
        Ok(())
    }
}

/// The name of a namespace: letters, digits and separators.
const NAMESPACE_REGEX: &str = r"([%pLN_][-%pLN_.:\\/]*)";

/// Enters the namespace group `quark`, with its stereotype, link and colour.
fn goto_namespace(
    cuca: &mut CucaDiagram,
    location: &LineLocation,
    quark: QuarkId,
    display: Display,
    arg: &RegexResult,
    usymbol: Option<crate::decoration::symbol::USymbol>,
) -> CommandResult {
    cuca.goto_group(Some(location), quark, display, GroupType::Package);
    let back = back_color(arg)?;
    let group = cuca.get_current_group();
    let entity = cuca.entity_mut(group);
    if usymbol.is_some() {
        entity.usymbol = usymbol;
    }
    if let Some(stereotype) = arg.get("STEREOTYPE", 0).filter(|_| usymbol.is_none()) {
        entity.stereotype = Some(Stereotype::new(stereotype));
    }
    if let Some(url) = url_of(arg) {
        entity.url = Some(url);
    }
    entity.colors = entity.colors.with(ColorType::Back, back);
    Ok(())
}

/// The end of a namespace declaration: stereotype, link, colour and `{`, then `}` for an empty one.
fn namespace_tail(empty: bool) -> Vec<RegexTree> {
    let mut parts = vec![
        stereo::optional_pattern("STEREOTYPE"),
        Url::optional_pattern(),
        RegexTree::spaces_zero_or_more(),
        color::optional_pattern("COLOR"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(r"\{"),
    ];
    if empty {
        parts.extend([RegexTree::spaces_zero_or_more(), RegexTree::leaf(r"\}")]);
    }
    parts.push(RegexTree::end());
    parts
}

/// PlantUML's `CommandNamespace`: `namespace net.foo {`.
pub(super) fn namespace() -> Box<dyn Command<ClassDiagram>> {
    let mut parts = vec![
        RegexTree::start(),
        RegexTree::leaf(r"namespace"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "NAME", NAMESPACE_REGEX),
    ];
    parts.extend(namespace_tail(false));
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(parts),
        |diagram: &mut ClassDiagram, location: &LineLocation, arg: &RegexResult| {
            let cuca = diagram.cuca();
            let name = arg.get("NAME", 0).unwrap_or_default();
            let quark = cuca.quark_in_context(false, ClassDiagram::clean_id(name))?;
            let skin = cuca.skin();
            let usymbol = arg.get("STEREOTYPE", 0).and_then(|stereotype| {
                USymbols::from_string(
                    stereotype,
                    skin.actor_style(),
                    skin.component_style(),
                    skin.package_style(),
                )
            });
            let display = Display::with_newlines(cuca.quark(quark).get_name());
            goto_namespace(cuca, location, quark, display, arg, usymbol)
        },
    )))
}

/// PlantUML's `CommandNamespace2`: `namespace "Display" as net.foo {`.
pub(super) fn namespace2() -> Box<dyn Command<ClassDiagram>> {
    let mut parts = vec![
        RegexTree::start(),
        RegexTree::leaf(r"namespace"),
        RegexTree::spaces_one_or_more(),
        RegexTree::leaf(r"[%g]"),
        RegexTree::named(1, "DISPLAY", r"([^%g]+)"),
        RegexTree::leaf(r"[%g]"),
        RegexTree::spaces_one_or_more(),
        RegexTree::leaf(r"as"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "NAME", NAMESPACE_REGEX),
    ];
    parts.extend(namespace_tail(false));
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(parts),
        |diagram: &mut ClassDiagram, location: &LineLocation, arg: &RegexResult| {
            let cuca = diagram.cuca();
            let name = arg.get("NAME", 0).unwrap_or_default();
            let quark = cuca.quark_in_context(false, ClassDiagram::clean_id(name))?;
            let display = Display::with_newlines(arg.get("DISPLAY", 0).unwrap_or_default());
            goto_namespace(cuca, location, quark, display, arg, None)
        },
    )))
}

/// PlantUML's `CommandNamespaceEmpty`: `namespace net.foo {}`.
pub(super) fn namespace_empty() -> Box<dyn Command<ClassDiagram>> {
    let mut parts = vec![
        RegexTree::start(),
        RegexTree::leaf(r"namespace"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "NAME", NAMESPACE_REGEX),
    ];
    parts.extend(namespace_tail(true));
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(parts),
        |diagram: &mut ClassDiagram, location: &LineLocation, arg: &RegexResult| {
            let cuca = diagram.cuca();
            let name = arg.get("NAME", 0).unwrap_or_default();
            let quark = cuca.quark_in_context(false, ClassDiagram::clean_id(name))?;
            if cuca.quark(quark).get_data().is_some() {
                return Err(CommandError::new(format!(
                    "Already exists {}",
                    cuca.quark(quark).get_name()
                )));
            }
            let display = Display::with_newlines(cuca.quark(quark).get_qualified_name());
            goto_namespace(cuca, location, quark, display, arg, None)?;
            cuca.end_group();
            Ok(())
        },
    )))
}

/// PlantUML's `CommandStereotype`: `Foo <<Stereo>>`.
pub(super) fn stereotype_command() -> Box<dyn Command<ClassDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "NAME", r"([%pLN_.]+|[%g][^%g]+[%g])"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "STEREO", r"(\<\<.+?\>\>)"),
            RegexTree::end(),
        ]),
        |diagram: &mut ClassDiagram, _: &LineLocation, arg: &RegexResult| {
            let name = without_quotes_or_brackets(arg.get("NAME", 0).unwrap_or_default());
            let cuca = diagram.cuca();
            let quark = cuca.quark_in_context(true, ClassDiagram::clean_id(name))?;
            let Some(entity) = cuca.quark(quark).get_data() else {
                return Err(CommandError::new(format!(
                    "No such class {}",
                    cuca.quark(quark).get_name()
                )));
            };
            let stereo = Stereotype::with_spot(arg.get("STEREO", 0).unwrap_or_default())?;
            cuca.entity_mut(entity).stereotype = Some(stereo);
            Ok(())
        },
    )))
}

/// A name in a link: letters and digits separated by punctuation or `::`, or anything quoted.
const LINK_ENTITY: &str = r"((?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_$]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_$]+)*|[%g][^%g]+[%g])";
/// `(A, B)`: the link between two classes, for an association class.
const LINK_COUPLE: &str = r"\([%s]*((?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_]+)*|[%g](?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_]+)*[%g]))[%s]*,[%s]*((?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_]+)*|[%g](?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_]+)*[%g]))[%s]*\)";

/// PlantUML's `CommandLinkClass`: `A "1" *-- "many" B : label`, and association classes `(A, B) .. C`.
pub(super) fn link_class() -> Box<dyn Command<ClassDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "HEADER", r"@([\d.]+)"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::or(vec![
                RegexTree::named(1, "ENT1", LINK_ENTITY),
                RegexTree::named(2, "COUPLE1", LINK_COUPLE),
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
            RegexTree::optional(RegexTree::named(
                1,
                "FIRST_ROLE",
                r"/([^%s]+|[%g][^%g]+[%g])",
            )),
            RegexTree::spaces_zero_or_more(),
            RegexTree::concat(vec![
                RegexTree::named(1, "ARROW_HEAD1", LinkDecor::get_regex_decors1()),
                RegexTree::named(1, "ARROW_BODY1", r"([-=.]+)"),
                RegexTree::named(1, "ARROW_STYLE1", arrow_style()),
                RegexTree::named(
                    1,
                    "ARROW_DIRECTION",
                    r"(left|right|up|down|le?|ri?|up?|do?)?",
                ),
                RegexTree::optional(RegexTree::named(
                    1,
                    "INSIDE",
                    r"(0|\(0\)|\(0|0\))(?=[-=.~])",
                )),
                RegexTree::named(1, "ARROW_STYLE2", arrow_style()),
                RegexTree::named(1, "ARROW_BODY2", r"([-=.]*)"),
                RegexTree::named(1, "ARROW_HEAD2", LinkDecor::get_regex_decors2()),
            ]),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(1, "SECOND_LABEL", r"[%g]([^%g]+)[%g]")),
            RegexTree::optional(RegexTree::named(
                1,
                "SECOND_ROLE",
                r"/([^%s]+|[%g][^%g]+[%g])",
            )),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"[\[]"),
                RegexTree::named(1, "QUALIFIER2", r"([^\[\]]+)"),
                RegexTree::leaf(r"[\]]"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::or(vec![
                RegexTree::named(1, "ENT2", LINK_ENTITY),
                RegexTree::named(2, "COUPLE2", LINK_COUPLE),
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
        execute_link_class,
    )))
}

fn execute_link_class(
    diagram: &mut ClassDiagram,
    location: &LineLocation,
    arg: &RegexResult,
) -> CommandResult {
    let ent1 = arg.get("ENT1", 0).map(ClassDiagram::clean_id);
    let ent2 = arg.get("ENT2", 0).map(ClassDiagram::clean_id);
    let (mut ent1_string, mut ent2_string) = match (ent1, ent2) {
        (None, None) => return execute_arg_special3(location, diagram, arg),
        (None, Some(_)) => return execute_arg_special(location, diagram, arg, 1),
        (Some(_), None) => return execute_arg_special(location, diagram, arg, 2),
        (Some(ent1), Some(ent2)) => (ent1.to_owned(), ent2.to_owned()),
    };
    let cuca = diagram.cuca();
    let link_type = get_link_type(arg);
    if ent1_string.contains("::") && cuca.first_with_name(&ent1_string).is_none() {
        ent1_string = cuca.remove_port_id(&ent1_string).to_owned();
    }
    if ent2_string.contains("::") && cuca.first_with_name(&ent2_string).is_none() {
        ent2_string = cuca.remove_port_id(&ent2_string).to_owned();
    }
    let quark1 = cuca
        .quark_in_context(true, &ent1_string)
        .map_err(|error| CommandError::new(error.message))?;
    let quark2 = cuca
        .quark_in_context(true, &ent2_string)
        .map_err(|error| CommandError::new(error.message))?;
    let cl1 = get_or_create_class(cuca, location, quark1);
    let cl2 = get_or_create_class(cuca, location, quark2);
    let dir = get_direction(arg);
    let queue = if matches!(dir, Direction::Left | Direction::Right) {
        1
    } else {
        get_queue_length(arg)
    };
    let labels = Labels::new(arg);
    let manage_visibility = cuca.skin().class_attribute_icon_size() > 0;
    let label = labels.get_label_link().map(Display::with_newlines);
    let link_arg = LinkArg::build_managing(label, queue, manage_visibility)
        .with_quantifier(labels.get_first_label(), labels.get_second_label())
        .with_role(labels.get_first_role(), labels.get_second_role());
    let mut link = cuca.new_link(Some(location), cl1, cl2, link_type, link_arg);
    cuca.link_mut(link).url = url_of(arg);
    if matches!(dir, Direction::Left | Direction::Up) {
        link = cuca.get_inv(link);
    }
    let colors = colors(arg, ColorType::Back)?;
    let stereotype = arg.get("STEREOTYPE", 0).map(crate::stereo::Stereotype::new);
    let link_mut: &mut Link = cuca.link_mut(link);
    link_mut.link_arrow = labels.get_link_arrow();
    link_mut.set_colors(colors);
    link_mut.apply_style(arg.get_lazzy("ARROW_STYLE", 0));
    if stereotype.is_some() {
        link_mut.stereotype = stereotype;
    }
    link_mut.code_line = Some(location.clone());
    cuca.add_link(link);
    if let Some(weight) = arg.get("HEADER", 0) {
        cuca.link_mut(link).weight = weight.parse().unwrap_or_default();
    }
    Ok(())
}

/// The entity `quark` holds, or a new class named after it.
fn get_or_create_class(
    cuca: &mut CucaDiagram,
    location: &LineLocation,
    quark: QuarkId,
) -> EntityId {
    get_or_create(cuca, location, quark, LeafType::Class)
}

/// The entity `quark` holds, or a new leaf of `leaf_type` named after it.
fn get_or_create(
    cuca: &mut CucaDiagram,
    location: &LineLocation,
    quark: QuarkId,
    leaf_type: LeafType,
) -> EntityId {
    if let Some(entity) = cuca.quark(quark).get_data() {
        return entity;
    }
    let display = Display::with_newlines(cuca.quark(quark).get_name());
    cuca.really_create_leaf(Some(location), quark, display, leaf_type)
}

/// The two entities of `(A, B)`, which must exist.
fn couple(
    cuca: &mut CucaDiagram,
    arg: &RegexResult,
    key: &str,
) -> Result<(EntityId, EntityId), CommandError> {
    let mut entity = |index| {
        let name = without_quotes_or_brackets(arg.get(key, index).unwrap_or_default());
        let quark = cuca.quark_in_context(true, name)?;
        cuca.quark(quark)
            .get_data()
            .ok_or_else(|| CommandError::new(format!("No class {name}")))
    };
    let a = entity(0)?;
    let b = entity(1)?;
    Ok((a, b))
}

/// `(A, B) .. C` (`mode` 1) or `C .. (A, B)` (`mode` 2).
fn execute_arg_special(
    location: &LineLocation,
    diagram: &mut ClassDiagram,
    arg: &RegexResult,
    mode: i32,
) -> CommandResult {
    let (couple_key, other_key) = if mode == 1 {
        ("COUPLE1", "ENT2")
    } else {
        ("COUPLE2", "ENT1")
    };
    let cuca = diagram.cuca();
    let (cl_a, cl_b) = couple(cuca, arg, couple_key)?;
    let id = unquoted(arg.get(other_key, 0).unwrap_or_default());
    let quark = cuca.quark_in_context(true, id)?;
    let other = get_or_create_class(cuca, location, quark);
    let label = arg.get("LABEL_LINK", 0).map(Display::with_newlines);
    diagram.diagram.association_class(
        Some(location),
        mode,
        cl_a,
        cl_b,
        other,
        get_link_type(arg),
        label,
    )
}

/// `(A, B) .. (C, D)`.
fn execute_arg_special3(
    location: &LineLocation,
    diagram: &mut ClassDiagram,
    arg: &RegexResult,
) -> CommandResult {
    let cuca = diagram.cuca();
    let couple1 = couple(cuca, arg, "COUPLE1")?;
    let couple2 = couple(cuca, arg, "COUPLE2")?;
    let label = arg.get("LABEL_LINK", 0).map(Display::with_newlines);
    diagram.diagram.association_class_between_links(
        Some(location),
        couple1,
        couple2,
        get_link_type(arg),
        label,
    )
}

fn get_link_type(arg: &RegexResult) -> LinkType {
    let decors1 = LinkDecor::lookup_decors1(Some(&get_arrow_head(arg, "ARROW_HEAD1")));
    let decors2 = LinkDecor::lookup_decors2(Some(&get_arrow_head(arg, "ARROW_HEAD2")));
    let mut result = LinkType::new(decors2, decors1);
    let body = |name| arg.get(name, 0).unwrap_or_default();
    if body("ARROW_BODY1").contains('.') || body("ARROW_BODY2").contains('.') {
        result = result.go_dashed();
    }
    match arg.get("INSIDE", 0) {
        Some("0") => result.with_middle_circle(),
        Some("0)") => result.with_middle_circle_circled1(),
        Some("(0") => result.with_middle_circle_circled2(),
        Some("(0)") => result.with_middle_circle_circled(),
        _ => result,
    }
}

fn get_arrow_head(arg: &RegexResult, key: &str) -> String {
    arg.get(key, 0).unwrap_or_default().replace('_', "")
}

fn get_full_arrow(arg: &RegexResult) -> String {
    let get = |name| arg.get(name, 0).unwrap_or_default();
    format!(
        "{}{}{}{}{}",
        get_arrow_head(arg, "ARROW_HEAD1"),
        get("ARROW_BODY1"),
        get("ARROW_DIRECTION"),
        get("ARROW_BODY2"),
        get_arrow_head(arg, "ARROW_HEAD2")
    )
}

fn get_queue_length(arg: &RegexResult) -> i32 {
    get_full_arrow(arg)
        .chars()
        .filter(|c| matches!(c, '-' | '.' | '='))
        .count() as i32
}

fn get_direction(arg: &RegexResult) -> Direction {
    let full: String = get_full_arrow(arg)
        .chars()
        .filter(|c| matches!(c, '-' | '.' | '=') || c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    let s = full.strip_prefix('o').unwrap_or(&full);
    let s = s.strip_suffix('o').unwrap_or(s);
    Direction::of_queue(s)
}

/// PlantUML's `CommandLinkLollipop`: `Foo ()- Bar` or `Foo -() Bar`, a lollipop interface named after the
/// other end.
pub(super) fn link_lollipop() -> Box<dyn Command<ClassDiagram>> {
    const ENTITY: &str = r"(?:(interface|enum|annotation|abstract[%s]+class|abstract|class|entity|protocol|struct|exception|metaclass|stereotype|dataclass|record)[%s]+)?((?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)?[%pLN_$]+(?:(?:[^%pLN%s_$#\:{}<>%g]|[\\]{2}|::)[%pLN_$]+)*|[%g][^%g]+[%g])[%s]*(\<\<.*\>\>)?";
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "HEADER", r"@([\d.]+)"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(3, "ENT1", ENTITY),
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
            RegexTree::named(3, "ENT2", ENTITY),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r":"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "LABEL_LINK", r"(.+)"),
            ])),
            RegexTree::end(),
        ]),
        execute_link_lollipop,
    )))
}

fn execute_link_lollipop(
    diagram: &mut ClassDiagram,
    location: &LineLocation,
    arg: &RegexResult,
) -> CommandResult {
    let ent1 = arg.get("ENT1", 1).unwrap_or_default();
    let ent2 = arg.get("ENT2", 1).unwrap_or_default();
    let cuca = diagram.cuca();
    let suffix = cuca.get_unique_sequence("lol");
    let lollipop_type = |desc: &str| {
        let mut chars = desc.chars();
        if chars.next() == chars.next() {
            LeafType::LollipopHalf
        } else {
            LeafType::LollipopFull
        }
    };
    let (normal, named, desc, lollipop_first) = match arg.get("LOL_THEN_ENT", 1) {
        None => (ent1, ent2, arg.get("ENT_THEN_LOL", 1), false),
        Some(_) => (ent2, ent1, arg.get("LOL_THEN_ENT", 0), true),
    };
    let quark = cuca.quark_in_context(true, ClassDiagram::clean_id(normal))?;
    let Some(normal_entity) = cuca.quark(quark).get_data() else {
        return Err(CommandError::new(format!(
            "No class {}",
            cuca.quark(quark).get_name()
        )));
    };
    let id_new_long =
        cuca.quark_in_context(true, &format!("{}{suffix}", ClassDiagram::clean_id(normal)))?;
    let lollipop = cuca.really_create_leaf(
        Some(location),
        id_new_long,
        Display::with_newlines(named),
        lollipop_type(desc.unwrap_or_default()),
    );
    let (cl1, cl2) = if lollipop_first {
        (lollipop, normal_entity)
    } else {
        (normal_entity, lollipop)
    };
    let queue = java::trim(
        arg.get("LOL_THEN_ENT", 1)
            .or_else(|| arg.get("ENT_THEN_LOL", 0))
            .unwrap_or_default(),
    );
    let mut length = queue.chars().count() as i32;
    if length == 1 && diagram.diagram.get_nb_of_hozizontal_lollipop(normal_entity) > 1 {
        length += 1;
    }
    let (first_label, label_link, second_label) = lollipop_labels(arg);
    let cuca = diagram.cuca();
    let manage_visibility = cuca.skin().class_attribute_icon_size() > 0;
    let link_arg = LinkArg::build_managing(
        label_link.as_deref().map(Display::with_newlines),
        length,
        manage_visibility,
    )
    .with_quantifier(first_label, second_label);
    let link = cuca.new_link(
        Some(location),
        cl1,
        cl2,
        LinkType::new(LinkDecor::None, LinkDecor::None),
        link_arg,
    );
    cuca.add_link(link);
    if let Some(weight) = arg.get("HEADER", 0) {
        cuca.link_mut(link).weight = weight.parse().unwrap_or_default();
    }
    Ok(())
}

/// The quantifiers and label of a lollipop link: `"1" label "2"` after the colon may hold quantifiers.
fn lollipop_labels(arg: &RegexResult) -> (Option<String>, Option<String>, Option<String>) {
    static BOTH: LazyLock<Regex> =
        LazyLock::new(|| plantuml_regex(r#"^"([^"]+)"([^"]+)"([^"]+)"$"#));
    static FIRST: LazyLock<Regex> = LazyLock::new(|| plantuml_regex(r#"^"([^"]+)"([^"]+)$"#));
    static SECOND: LazyLock<Regex> = LazyLock::new(|| plantuml_regex(r#"^([^"]+)"([^"]+)"$"#));
    let mut first_label = arg.get("FIRST_LABEL", 0).map(str::to_owned);
    let mut second_label = arg.get("SECOND_LABEL", 0).map(str::to_owned);
    let Some(label) = arg.get("LABEL_LINK", 0) else {
        return (first_label, None, second_label);
    };
    let middle = |text: &str| java::trim(without_quotes_or_brackets(java::trim(text))).to_owned();
    let mut label_link = label.to_owned();
    if first_label.is_none() && second_label.is_none() {
        if let Some(m) = BOTH.captures(label) {
            first_label = Some(m[1].to_owned());
            label_link = middle(&m[2]);
            second_label = Some(m[3].to_owned());
        } else if let Some(m) = FIRST.captures(label) {
            first_label = Some(m[1].to_owned());
            label_link = middle(&m[2]);
            second_label = None;
        } else if let Some(m) = SECOND.captures(label) {
            first_label = None;
            label_link = middle(&m[1]);
            second_label = Some(m[2].to_owned());
        }
    }
    (
        first_label,
        Some(without_quotes_or_brackets(&label_link).to_owned()),
        second_label,
    )
}

/// PlantUML's `CommandDiamondAssociation`: `<> name`.
pub(super) fn diamond_association() -> Box<dyn Command<ClassDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"\<\>"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "CODE", r"([%pLN_.]+)"),
            RegexTree::end(),
        ]),
        |diagram: &mut ClassDiagram, location: &LineLocation, arg: &RegexResult| {
            let cuca = diagram.cuca();
            let code = arg.get("CODE", 0).unwrap_or_default();
            let quark = cuca.quark_in_context(true, ClassDiagram::clean_id(code))?;
            if cuca.quark(quark).get_data().is_some() {
                return Err(CommandError::new(format!(
                    "Already existing : {}",
                    cuca.quark(quark).get_name()
                )));
            }
            cuca.really_create_leaf(
                Some(location),
                quark,
                Display::with_newlines(""),
                LeafType::Association,
            );
            Ok(())
        },
    )))
}
