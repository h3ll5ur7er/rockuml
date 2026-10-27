use std::rc::Rc;

use super::*;
use crate::command::factory::{Created, create_system};
use crate::klimt::debug::StringBounderDebug;
use crate::text::StringLocated;

fn parse(body: &[&str]) -> Result<ActivityDiagram, String> {
    let lines: Vec<&str> = ["@startuml"]
        .iter()
        .chain(body)
        .chain(&["@enduml"])
        .copied()
        .collect();
    parse_source(&lines)
}

/// The diagram the lines from `@startuml` to `@enduml` make.
pub(super) fn parse_source(source: &[&str]) -> Result<ActivityDiagram, String> {
    let location = LineLocation::new("test", None);
    let lines: Vec<StringLocated> = source
        .iter()
        .map(|text| StringLocated::new(*text, location.clone()))
        .collect();
    let source = Rc::new(UmlSource::new(lines, Vec::new()));
    let commands = ActivityDiagramFactory::init_commands_list();
    match create_system(
        &source,
        || ActivityDiagramFactory::create_empty_diagram(&source),
        &commands,
    ) {
        Created::Diagram(diagram) => Ok(diagram),
        Created::Failure(failure) => Err(failure.error.message),
        Created::Nothing => Err("nothing".to_owned()),
    }
}

/// Each entity as `uid name type`, in creation order.
fn entities(diagram: &ActivityDiagram) -> Vec<String> {
    let cuca = &diagram.cuca;
    cuca.leafs()
        .into_iter()
        .chain(cuca.groups())
        .map(|id| {
            let entity = cuca.entity(id);
            let kind = match entity.get_leaf_type() {
                Some(leaf_type) => leaf_type.name().to_owned(),
                None => format!("{:?}", entity.get_group_type()),
            };
            let name = cuca.quark(entity.get_quark()).get_qualified_name();
            format!("{} {name} {kind}", entity.get_uid())
        })
        .collect()
}

/// Each link as `from->to length`.
fn links(diagram: &ActivityDiagram) -> Vec<String> {
    let cuca = &diagram.cuca;
    let name = |id| cuca.quark(cuca.entity(id).get_quark()).get_qualified_name();
    cuca.get_links()
        .map(|link| {
            format!(
                "{}->{} {}",
                name(link.get_entity1()),
                name(link.get_entity2()),
                link.get_length()
            )
        })
        .collect()
}

#[test]
fn a_link_without_source_starts_from_the_latest_activity() {
    let diagram = parse(&[r#"(*) --> "First""#, r#"-right-> "Second""#, "-up-> (*)"]).unwrap();
    assert_eq!(
        entities(&diagram),
        [
            "ent0001 start CIRCLE_START",
            "ent0002 First ACTIVITY",
            "ent0004 Second ACTIVITY",
            "ent0006 end CIRCLE_END",
        ]
    );
    assert_eq!(
        links(&diagram),
        ["start->First 2", "First->Second 1", "end->Second 2"]
    );
}

#[test]
fn an_if_opens_a_branch_its_else_starts_from() {
    let diagram = parse(&[
        r#"(*) --> if "test" then"#,
        r#"-->[yes] "A""#,
        "else",
        r#"->[no] "B""#,
        "endif",
    ])
    .unwrap();
    assert_eq!(links(&diagram), ["start->#2 2", "#2->A 2", "#2->B 1"]);
    let if_link = diagram.cuca.get_links().next().unwrap();
    assert_eq!(if_link.get_quantifier2(), Some("test"));
}

#[test]
fn else_and_endif_need_an_if() {
    assert_eq!(
        parse(&[r#"(*) --> "A""#, "else"]).err().as_deref(),
        Some("No if for this else")
    );
    assert_eq!(
        parse(&[r#"(*) --> "A""#, "endif"]).err().as_deref(),
        Some("No if for this endif")
    );
}

#[test]
fn a_quoted_activity_may_span_lines_and_take_a_name() {
    let diagram = parse(&[r#"(*) --> "one"#, r#"two" as T"#, "T --> (*)"]).unwrap();
    let cuca = &diagram.cuca;
    let named = cuca
        .quark(cuca.first_with_name("T").unwrap())
        .get_data()
        .unwrap();
    assert_eq!(
        cuca.entity(named).display.lines(),
        ["one".to_owned(), "two".to_owned()]
    );
    assert_eq!(links(&diagram), ["start->T 2", "T->end 2"]);
}

#[test]
fn a_note_goes_on_the_latest_activity() {
    let diagram = parse(&[r#"(*) --> "A""#, "note left: hello"]).unwrap();
    assert_eq!(links(&diagram), ["start->A 2", "GN4->A 1"]);
}

#[test]
fn inner_activities_are_not_drawn_yet() {
    let diagram = parse(&["(*) --> {", r#"--> "A""#, "}"]).unwrap();
    let string_bounder: Rc<dyn StringBounder> = Rc::new(StringBounderDebug);
    assert!(diagram.text_block(0, &string_bounder).is_err());
}
