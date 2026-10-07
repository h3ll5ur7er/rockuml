//! The commands of Chen entity relationship diagrams. The entities, attributes and their blocks are read,
//! because an attribute outside a block is an error; the other commands only recognise their lines.

use super::ChenEerDiagram;
use crate::abel::{LeafType, LinkArg};
use crate::color::{self, ColorType, Colors};
use crate::command::unported::{self, NotPortedCommands};
use crate::command::{Command, CommandError, PatternCommand, SingleLine};
use crate::creole::Display;
use crate::decoration::{LinkDecor, LinkType};
use crate::diagram::cuca::CucaDiagram;
use crate::java;
use crate::pattern::{RegexResult, RegexTree};
use crate::stereo::Stereotype;
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
                cuca.entity_mut(entity).stereotype = Some(Stereotype::new(stereo));
            }
            cuca.entity_mut(entity).colors = colors(arg, ColorType::Back)?;
            diagram.owner_stack.push(entity);
            diagram.command_not_ported("CommandCreateEntity");
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
                cuca.entity_mut(entity).stereotype = Some(Stereotype::new(stereo));
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
            diagram.command_not_ported("CommandCreateAttribute");
            Ok(())
        },
    )))
}

/// PlantUML's `CommandAssociate`.
pub(super) fn associate<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandAssociate",
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
    )
    .boxed()
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

/// The colours a `COLOR` specification gives, the main one painting `main_type`.
fn colors(arg: &RegexResult, main_type: ColorType) -> Result<Colors, CommandError> {
    arg.get("COLOR", 0)
        .map(|data| Colors::parse(data, main_type).map_err(|_| CommandError::bad_color()))
        .transpose()
        .map(Option::unwrap_or_default)
}

/// PlantUML's `CommandSimpleSubclass`.
pub(super) fn simple_subclass<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandSimpleSubclass",
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
    )
    .boxed()
}

/// PlantUML's `CommandMultiSubclass`.
pub(super) fn multi_subclass<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandMultiSubclass",
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
    )
    .boxed()
}
