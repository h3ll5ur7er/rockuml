//! The commands of Chen entity relationship diagrams (PlantUML's `cheneer.command` package).

use super::ChenEerDiagram;
use crate::abel::{EntityId, LeafType, LinkArg};
use crate::color::{self, ColorType, Colors};
use crate::command::{Command, CommandError, PatternCommand, SingleLine};
use crate::creole::Display;
use crate::decoration::{LinkDecor, LinkType};
use crate::diagram::cuca::CucaDiagram;
use crate::java;
use crate::pattern::{RegexResult, RegexTree};
use crate::text::LineLocation;

/// PlantUML's `CommandCreateEntity`: `entity Movie {` or `relationship Rents {`, which own the attributes
/// up to their `}`.
pub(super) fn create_entity() -> Box<dyn Command<ChenEerDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "TYPE", "(entity|relationship)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "DISPLAY", "[%g]([^%g]+)[%g]"),
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf("as"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(1, "CODE", "([%pLN_.]+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "STEREO", "(<<.+>>)?"),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{"),
            RegexTree::end(),
        ]),
        |diagram: &mut ChenEerDiagram, location: &LineLocation, arg: &RegexResult| {
            let leaf_type = if arg.get("TYPE", 0) == Some("entity") {
                LeafType::ChenEntity
            } else {
                LeafType::ChenRelationship
            };
            let cuca = &mut diagram.cuca;
            let id_short = arg.get("CODE", 0).unwrap_or_default();
            let quark = cuca.quark_in_context(true, CucaDiagram::clean_id(id_short));
            let entity = if let Some(entity) = cuca.quark(quark).get_data() {
                if !cuca
                    .entity_mut(entity)
                    .mute_to_type_if_compatible(leaf_type)
                {
                    return Err(CommandError::new("Bad name"));
                }
                entity
            } else {
                let display_text = arg
                    .get("DISPLAY", 0)
                    .unwrap_or_else(|| cuca.quark(quark).get_name())
                    .to_owned();
                cuca.really_create_leaf(
                    Some(location),
                    quark,
                    Display::with_newlines(&display_text),
                    leaf_type,
                )
            };
            if let Some(stereo) = arg.get("STEREO", 0) {
                cuca.entity_mut(entity)
                    .set_stereotype_and_stereostyle(stereo);
            }
            cuca.entity_mut(entity).colors = colors(arg, ColorType::Back)?;
            diagram.owner_stack.push(entity);
            Ok(())
        },
    )))
}

/// PlantUML's `CommandCreateAttribute`: an attribute of the entity, relationship or composite attribute
/// whose block it is in.
pub(super) fn create_attribute() -> Box<dyn Command<ChenEerDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "DISPLAY", "[%g]([^%g]+)[%g]"),
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf("as"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(1, "CODE", "([%pLN%s_.:]+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "STEREO", "(<<.*>>)?"),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "COMPOSITE", r"(\{)?"),
            RegexTree::end(),
        ]),
        |diagram: &mut ChenEerDiagram, location: &LineLocation, arg: &RegexResult| {
            let Some(&owner) = diagram.owner_stack.last() else {
                return Err(CommandError::new(
                    "Attribute must be inside an entity, relationship or another attribute",
                ));
            };
            let cuca = &mut diagram.cuca;
            let id_short =
                CucaDiagram::clean_id(java::trim(arg.get("CODE", 0).unwrap_or_default()))
                    .to_owned();
            let id = format!("{}/{id_short}", cuca.entity(owner).get_name(cuca));
            let quark = cuca.quark_in_context(true, &id);
            if cuca.quark(quark).get_data().is_some() {
                return Err(CommandError::new("Attribute already exists"));
            }
            let colors = colors(arg, ColorType::Line)?;
            let display_text = arg.get("DISPLAY", 0).unwrap_or(&id_short);
            let entity = cuca.really_create_leaf(
                Some(location),
                quark,
                Display::with_newlines(display_text),
                LeafType::ChenAttribute,
            );
            if let Some(stereo) = arg.get("STEREO", 0) {
                cuca.entity_mut(entity)
                    .set_stereotype_and_stereostyle(stereo);
            }
            cuca.entity_mut(entity).colors = colors.clone();
            let link = cuca.new_link(
                Some(location),
                entity,
                owner,
                LinkType::new(LinkDecor::None, LinkDecor::None),
                LinkArg::build(None, 2),
            );
            cuca.link_mut(link).set_colors(colors);
            cuca.add_link(link);
            if arg.get("COMPOSITE", 0).is_some() {
                diagram.owner_stack.push(entity);
            }
            Ok(())
        },
    )))
}

/// PlantUML's `CommandAssociate`: `Person -N- Rents`, a line between an entity and a relationship with its
/// cardinality; `=` for total participation.
pub(super) fn associate() -> Box<dyn Command<ChenEerDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "NAME1", r"([%pLN_.-]+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "PARTICIPATION", r"([-=])"),
            RegexTree::optional(RegexTree::named(
                1,
                "CARDINALITY",
                r"([%pLN]+|\([%pLN]+,[%s]*[%pLN]+\))",
            )),
            RegexTree::named(1, "PARTICIPATION2", r"([-=])"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "NAME2", r"([%pLN_.-]+)"),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::end(),
        ]),
        |diagram: &mut ChenEerDiagram, location: &LineLocation, arg: &RegexResult| {
            let cuca = &mut diagram.cuca;
            let entity1 = existing_entity(cuca, arg.get("NAME1", 0).unwrap_or_default())?;
            let entity2 = existing_entity(cuca, arg.get("NAME2", 0).unwrap_or_default())?;
            let mut link_type = LinkType::new(LinkDecor::None, LinkDecor::None);
            if is_double(arg) {
                link_type = link_type.go_bold();
            }
            let cardinality = arg.get("CARDINALITY", 0).map(Display::with_newlines);
            let colors = colors(arg, ColorType::Line)?;
            add_link(
                cuca,
                location,
                (entity1, entity2),
                link_type,
                LinkArg::build(cardinality, 3),
                colors,
                true,
            );
            Ok(())
        },
    )))
}

/// PlantUML's `CommandEndGroup`: `}` closes the block of the latest owner.
pub(super) fn end_group() -> Box<dyn Command<ChenEerDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"\}"),
            RegexTree::end(),
        ]),
        |diagram: &mut ChenEerDiagram, _: &LineLocation, _: &RegexResult| {
            diagram
                .owner_stack
                .pop()
                .map(|_| ())
                .ok_or_else(|| CommandError::new("Unbalanced brackets"))
        },
    )))
}

/// PlantUML's `CommandSimpleSubclass`: `Person ->- Student`, a subset (`<`) or superset (`>`) line.
pub(super) fn simple_subclass() -> Box<dyn Command<ChenEerDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "NAME1", r"([%pLN_.-]+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "PARTICIPATION", r"([-=])"),
            RegexTree::named(1, "DIRECTION", r"([<>])"),
            RegexTree::named(1, "PARTICIPATION2", r"([-=])"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "NAME2", r"([%pLN_.-]+)"),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::end(),
        ]),
        |diagram: &mut ChenEerDiagram, location: &LineLocation, arg: &RegexResult| {
            let cuca = &mut diagram.cuca;
            let entity1 = existing_entity(cuca, arg.get("NAME1", 0).unwrap_or_default())?;
            let entity2 = existing_entity(cuca, arg.get("NAME2", 0).unwrap_or_default())?;
            let mut link_type = LinkType::new(LinkDecor::None, LinkDecor::None);
            if is_double(arg) {
                link_type = link_type.go_bold();
            }
            link_type = if arg.get("DIRECTION", 0) == Some(">") {
                link_type.with_middle_superset()
            } else {
                link_type.with_middle_subset()
            };
            let colors = colors(arg, ColorType::Line)?;
            add_link(
                cuca,
                location,
                (entity1, entity2),
                link_type,
                LinkArg::build(None, 3),
                colors,
                true,
            );
            Ok(())
        },
    )))
}

/// PlantUML's `CommandMultiSubclass`: `Person =>= d { Student, Teacher }`, subclasses hanging from a circle
/// with `d` (disjoint), `o` (overlapping) or `U` (union) in it.
pub(super) fn multi_subclass() -> Box<dyn Command<ChenEerDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "SUPERCLASS", r"([%pLN_.-]+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "PARTICIPATION", r"([-=])"),
            RegexTree::counted(1, r"(>)"),
            RegexTree::named(1, "PARTICIPATION2", r"([-=])"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "SYMBOL", r"([doU])"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{"),
            RegexTree::named(1, "SUBCLASSES", r"(.+)"),
            RegexTree::leaf(r"\}"),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::end(),
        ]),
        |diagram: &mut ChenEerDiagram, location: &LineLocation, arg: &RegexResult| {
            let cuca = &mut diagram.cuca;
            let superclass =
                CucaDiagram::clean_id(arg.get("SUPERCLASS", 0).unwrap_or_default()).to_owned();
            let subclasses = arg.get("SUBCLASSES", 0).unwrap_or_default();
            let symbol = arg.get("SYMBOL", 0).unwrap_or_default();
            let colors = colors(arg, ColorType::Back)?;
            let center_quark =
                cuca.quark_in_context(false, &format!("{superclass}/{symbol}{subclasses}/center"));
            if cuca.quark(center_quark).get_data().is_some() {
                return Err(CommandError::new("Subclasses already exist"));
            }
            let center = cuca.really_create_leaf(
                Some(location),
                center_quark,
                Display::create([symbol]),
                LeafType::ChenCircle,
            );
            cuca.entity_mut(center).colors = colors.clone();
            let superclass_entity = existing_entity(cuca, &superclass)?;
            let mut link_type = LinkType::new(LinkDecor::None, LinkDecor::None);
            if is_double(arg) {
                link_type = link_type.go_bold();
            }
            if symbol == "U" {
                link_type = link_type.with_middle_superset();
            }
            add_link(
                cuca,
                location,
                (superclass_entity, center),
                link_type,
                LinkArg::build(None, 2),
                colors.clone(),
                true,
            );
            for subclass in java::split(subclasses, ",") {
                let subclass_entity =
                    existing_entity(cuca, CucaDiagram::clean_id(java::trim(&subclass)))?;
                let mut subclass_link_type = LinkType::new(LinkDecor::None, LinkDecor::None);
                if symbol != "U" {
                    subclass_link_type = subclass_link_type.with_middle_superset();
                }
                add_link(
                    cuca,
                    location,
                    (center, subclass_entity),
                    subclass_link_type,
                    LinkArg::build(None, 3),
                    colors.clone(),
                    false,
                );
            }
            Ok(())
        },
    )))
}

/// The entity a name written in a link means, which must exist.
fn existing_entity(cuca: &mut CucaDiagram, name: &str) -> Result<EntityId, CommandError> {
    let name = CucaDiagram::clean_id(name).to_owned();
    let quark = cuca.quark_in_context(true, &name);
    cuca.quark(quark)
        .get_data()
        .ok_or_else(|| CommandError::new(format!("No such entity: {name}")))
}

/// `=` draws total participation, a double line.
fn is_double(arg: &RegexResult) -> bool {
    arg.get("PARTICIPATION", 0) == Some("=")
}

/// Adds a link between two entities in `colors`; `with_ports` reads ports from their names, as links
/// written between two names do.
fn add_link(
    cuca: &mut CucaDiagram,
    location: &LineLocation,
    (entity1, entity2): (EntityId, EntityId),
    link_type: LinkType,
    link_arg: LinkArg,
    colors: Colors,
    with_ports: bool,
) {
    let link = cuca.new_link(Some(location), entity1, entity2, link_type, link_arg);
    if with_ports {
        let port = |entity: EntityId, cuca: &CucaDiagram| {
            cuca.get_port_id(cuca.entity(entity).get_name(cuca))
                .map(str::to_owned)
        };
        let (port1, port2) = (port(entity1, cuca), port(entity2, cuca));
        cuca.set_port_members(link, port1, port2);
    }
    cuca.link_mut(link).set_colors(colors);
    cuca.add_link(link);
}

/// The colours a `COLOR` specification gives, the main one painting `main_type`.
fn colors(arg: &RegexResult, main_type: ColorType) -> Result<Colors, CommandError> {
    arg.get("COLOR", 0)
        .map(|data| Colors::parse(data, main_type).map_err(|_| CommandError::bad_color()))
        .transpose()
        .map(Option::unwrap_or_default)
}
