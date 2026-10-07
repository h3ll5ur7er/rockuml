//! `hide` and `show` of portions of entities, by type, stereotype or name, and of members by visibility
//! (PlantUML's `CommandHideShowByGender` and `CommandHideShowByVisibility`). Every diagram reads these lines;
//! class and object diagrams apply them.

use crate::abel::{EntityGender, EntityPortion, LeafType};
use crate::command::unported::NotPortedCommands;
use crate::command::{Command, CommandError, CommandResult, PatternCommand, SingleLine};
use crate::diagram::cuca::CucaDiagram;
use crate::diagram::titled::TitledDiagram;
use crate::pattern::{RegexResult, RegexTree};
use crate::skin::visibility_modifier::VisibilityModifier;
use crate::text::LineLocation;

/// PlantUML's `CommandHideShowByGender`: `hide class circle`, `hide empty members`, `show Foo fields`...
pub(in crate::diagram) fn hide_show_by_gender<D: TitledDiagram + 'static>() -> Box<dyn Command<D>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "COMMAND", r"(hide|show)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(
                1,
                "GENDER",
                r"(?:(class|object|interface|enum|annotation|dataclass|record|abstract|[%pLN_.]+|[%g][^%g]+[%g]|\<\<.*\>\>)[%s]+)*?",
            ),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "EMPTY", r"(empty)"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(
                1,
                "PORTION",
                r"(members?|attributes?|fields?|methods?|circles?|circled?|stereotypes?)",
            ),
            RegexTree::end(),
        ]),
        |diagram: &mut D, _: &LineLocation, arg: &RegexResult| {
            if let Some(cuca) = diagram.class_or_object_diagram() {
                return execute_class_diagram(cuca, arg);
            }
            diagram.command_not_ported("CommandHideShowByGender");
            Ok(())
        },
    )))
}

fn get_entity_portion(s: &str) -> EntityPortion {
    match s.get(..3).unwrap_or_default().to_lowercase().as_str() {
        "met" => EntityPortion::Method,
        "mem" => EntityPortion::Member,
        "att" | "fie" => EntityPortion::Field,
        "cir" => EntityPortion::CircledCharacter,
        "ste" => EntityPortion::Stereotype,
        _ => unreachable!("the pattern only accepts portions"),
    }
}

fn empty_by_gender(portion: EntityPortion) -> EntityGender {
    match portion {
        EntityPortion::Method => EntityGender::EmptyMethods,
        EntityPortion::Field => EntityGender::EmptyFields,
        _ => EntityGender::All,
    }
}

fn and(gender1: EntityGender, gender2: EntityGender) -> EntityGender {
    EntityGender::And(Box::new(gender1), Box::new(gender2))
}

fn execute_class_diagram(cuca: &mut CucaDiagram, arg: &RegexResult) -> CommandResult {
    let portion = get_entity_portion(arg.get("PORTION", 0).unwrap_or_default());
    let mut gender = match arg.get("GENDER", 0) {
        None => EntityGender::All,
        Some(arg1) => match class_type(arg1) {
            Some(leaf_type) => EntityGender::ByEntityType(leaf_type),
            None if arg1.starts_with("<<") => EntityGender::ByStereotype(arg1.to_owned()),
            None => {
                let arg1 = CucaDiagram::clean_id(arg1);
                let quark = cuca.quark_in_context(true, CucaDiagram::clean_id(arg1))?;
                if portion == EntityPortion::Method {
                    EntityGender::ByClassName(arg1.to_owned())
                } else {
                    let Some(entity) = cuca.quark(quark).get_data() else {
                        return Err(CommandError::new(format!(
                            "No such element {}",
                            cuca.quark(quark).get_name()
                        )));
                    };
                    EntityGender::ByEntityAlone(entity)
                }
            }
        },
    };
    let empty = arg.get("EMPTY", 0).is_some();
    let empty_members = empty && portion == EntityPortion::Member;
    if empty && !empty_members {
        gender = and(gender, empty_by_gender(portion));
    }
    let current_group = cuca.get_current_group();
    if !cuca.entity(current_group).is_root() {
        gender = and(gender, EntityGender::ByPackage(current_group));
    }
    let show = arg
        .get("COMMAND", 0)
        .is_some_and(|command| command.eq_ignore_ascii_case("show"));
    if empty_members {
        for portion in [EntityPortion::Field, EntityPortion::Method] {
            let gender = and(gender.clone(), empty_by_gender(portion));
            cuca.hide_or_show(&gender, portion, show);
        }
    } else {
        cuca.hide_or_show(&gender, portion, show);
    }
    Ok(())
}

/// The type a `hide` keyword names.
fn class_type(keyword: &str) -> Option<LeafType> {
    Some(match keyword.to_lowercase().as_str() {
        "class" => LeafType::Class,
        "object" => LeafType::Object,
        "interface" => LeafType::Interface,
        "enum" => LeafType::Enum,
        "abstract" => LeafType::AbstractClass,
        "annotation" => LeafType::Annotation,
        "protocol" => LeafType::Protocol,
        "struct" => LeafType::Struct,
        "exception" => LeafType::Exception,
        "metaclass" => LeafType::Metaclass,
        "stereotype" => LeafType::Stereotype,
        "dataclass" => LeafType::Dataclass,
        "record" => LeafType::Record,
        _ => return None,
    })
}

/// PlantUML's `CommandHideShowByVisibility`: `hide private members`, `show public, protected methods`.
pub(in crate::diagram) fn hide_show_by_visibility<D: TitledDiagram + 'static>()
-> Box<dyn Command<D>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "COMMAND", r"(hide|show)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(
                1,
                "VISIBILITY",
                r"((?:public|private|protected|package)?(?:[,%s]+(?:public|private|protected|package))*)",
            ),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "PORTION", r"(members?|attributes?|fields?|methods?)"),
            RegexTree::end(),
        ]),
        |diagram: &mut D, _: &LineLocation, arg: &RegexResult| {
            // Only class diagrams have members.
            let Some(cuca) = diagram.class_or_object_diagram() else {
                return Ok(());
            };
            let portion = get_entity_portion(arg.get("PORTION", 0).unwrap_or_default());
            let fields = matches!(portion, EntityPortion::Member | EntityPortion::Field);
            let methods = matches!(portion, EntityPortion::Member | EntityPortion::Method);
            let mut visibilities = Vec::new();
            let tokens = arg.get("VISIBILITY", 0).unwrap_or_default().to_lowercase();
            for token in tokens.split([' ', ',']).filter(|token| !token.is_empty()) {
                let (field, method) = match token {
                    "public" => (
                        VisibilityModifier::PublicField,
                        VisibilityModifier::PublicMethod,
                    ),
                    "private" => (
                        VisibilityModifier::PrivateField,
                        VisibilityModifier::PrivateMethod,
                    ),
                    "protected" => (
                        VisibilityModifier::ProtectedField,
                        VisibilityModifier::ProtectedMethod,
                    ),
                    "package" => (
                        VisibilityModifier::PackagePrivateField,
                        VisibilityModifier::PackagePrivateMethod,
                    ),
                    _ => continue,
                };
                if fields {
                    visibilities.push(field);
                }
                if methods {
                    visibilities.push(method);
                }
            }
            let show = arg
                .get("COMMAND", 0)
                .is_some_and(|command| command.eq_ignore_ascii_case("show"));
            cuca.hide_or_show_visibility_modifier(&visibilities, show);
            Ok(())
        },
    )))
}
