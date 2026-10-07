use std::rc::Rc;

use super::*;
use crate::color::ColorType;
use crate::command::factory::{Created, create_system};
use crate::decoration::LinkDecor;
use crate::text::StringLocated;

fn parse(body: &[&str]) -> Result<StateDiagram, String> {
    let location = LineLocation::new("test", None);
    let lines: Vec<StringLocated> = ["@startuml"]
        .iter()
        .chain(body)
        .chain(&["@enduml"])
        .map(|text| StringLocated::new(*text, location.clone()))
        .collect();
    let source = Rc::new(UmlSource::new(lines, Vec::new()));
    let commands = StateDiagramFactory::init_commands_list();
    match create_system(
        &source,
        || StateDiagramFactory::create_empty_diagram(&source),
        &commands,
    ) {
        Created::Diagram(diagram) => Ok(diagram),
        Created::Failure(failure) => Err(failure.error.message),
        Created::Nothing => Err("nothing".to_owned()),
    }
}

/// Each entity as `uid qualified-name type`, in creation order.
fn entities(diagram: &StateDiagram) -> Vec<String> {
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

/// Each link as `uid from->to length`.
fn links(diagram: &StateDiagram) -> Vec<String> {
    let cuca = &diagram.cuca;
    let name = |id| cuca.quark(cuca.entity(id).get_quark()).get_qualified_name();
    cuca.get_links()
        .map(|link| {
            format!(
                "{} {}->{} {}",
                link.get_uid(),
                name(link.get_entity1()),
                name(link.get_entity2()),
                link.get_length()
            )
        })
        .collect()
}

#[test]
fn links_are_read_in_the_second_pass_after_every_state() {
    let diagram = parse(&[
        "[*] --> State1",
        "State1 --> [*]",
        "State1 : this is a string",
        "State1 -> State2",
        "state State2",
    ])
    .unwrap();
    assert_eq!(
        entities(&diagram),
        [
            "ent0001 State1 STATE",
            "ent0002 State2 STATE",
            "ent0003 *start* CIRCLE_START",
            "ent0005 *end* CIRCLE_END",
        ]
    );
    assert_eq!(
        links(&diagram),
        [
            "lnk4 *start*->State1 2",
            "lnk6 State1->*end* 2",
            "lnk7 State1->State2 1",
        ]
    );
    let state1 = diagram.cuca.get_group("State1").unwrap();
    assert_eq!(
        diagram.cuca.entity(state1).bodier.get_raw_body(),
        ["this is a string"]
    );
}

#[test]
fn composite_states_have_their_own_pseudo_states() {
    let diagram = parse(&[
        "[*] --> Outer",
        "state Outer {",
        "  [*] --> Inner",
        "  Inner --> [*]",
        "}",
    ])
    .unwrap();
    assert_eq!(
        entities(&diagram),
        [
            "ent0002 *start* CIRCLE_START",
            "ent0004 Outer.*start*Outer CIRCLE_START",
            "ent0005 Outer.Inner STATE",
            "ent0007 Outer.*end*Outer CIRCLE_END",
            "ent0001 Outer State",
        ]
    );
}

#[test]
fn concurrent_regions_are_numbered_afresh_in_each_pass() {
    let diagram = parse(&[
        "state Active {",
        "  [*] -> A",
        "  --",
        "  [*] -> B",
        "  ||",
        "  [*] -> C",
        "}",
    ])
    .unwrap();
    let names: Vec<String> = entities(&diagram)
        .into_iter()
        .map(|entity| entity.split_once(' ').unwrap().1.to_owned())
        .collect();
    assert_eq!(
        names,
        [
            "Active.*start*Active CIRCLE_START",
            "Active.A STATE",
            "Active.CONC1.*start*CONC1 CIRCLE_START",
            "Active.CONC1.B STATE",
            "Active.CONC2.*start*CONC2 CIRCLE_START",
            "Active.CONC2.C STATE",
            "Active State",
            "Active.CONC1 ConcurrentState",
            "Active.CONC2 ConcurrentState",
        ]
    );
    let cuca = &diagram.cuca;
    let active = cuca.get_group("Active").unwrap();
    assert_eq!(cuca.entity(active).concurrent_separator, Some('-'));
    let conc2 = cuca.get_group("CONC2").unwrap();
    assert_eq!(cuca.entity(conc2).concurrent_separator, Some('|'));
    let conc1 = cuca.get_group("CONC1").unwrap();
    assert_eq!(cuca.entity(conc1).concurrent_separator, Some('|'));
}

#[test]
fn states_of_a_concurrent_region_are_not_used_outside() {
    let error = parse(&[
        "state Active {",
        "  A --> B",
        "  --",
        "  C --> D",
        "}",
        "A --> D",
    ])
    .err();
    assert_eq!(error.as_deref(), Some("The state D cannot be used here."));
}

#[test]
fn stereotypes_make_pseudo_states_and_place_pins() {
    let diagram = parse(&[
        "state c <<choice>>",
        "state f <<fork>>",
        "state h <<history>>",
        "state S {",
        "  state e1 <<entryPoint>>",
        "}",
    ])
    .unwrap();
    let kinds: Vec<String> = entities(&diagram)
        .into_iter()
        .map(|entity| entity.split_once(' ').unwrap().1.to_owned())
        .collect();
    assert_eq!(
        kinds,
        [
            "c STATE_CHOICE",
            "f STATE_FORK_JOIN",
            "h PSEUDO_STATE",
            "S.e1 STATE",
            "S State"
        ]
    );
    assert_eq!(
        parse(&["state e1 <<entryPoint>>"]).err().as_deref(),
        Some("You cannot use this stereotype here")
    );
}

#[test]
fn history_of_a_state_makes_it_composite() {
    let diagram = parse(&["Paused --> Working[H]", "Paused --> Working[H*]"]).unwrap();
    assert_eq!(
        links(&diagram),
        [
            "lnk4 Paused->Working.*historical*Working 2",
            "lnk6 Paused->Working.*deephistory*Working 2",
        ]
    );
    let working = diagram.cuca.get_group("Working").unwrap();
    assert_eq!(
        diagram.cuca.entity(working).get_group_type(),
        GroupType::State
    );
}

#[test]
fn arrows_set_decorations_direction_and_colours() {
    let diagram = parse(&[
        "state Blue #lightblue ##[dashed]blue",
        "A -up-> B",
        "A x-left->o B",
        "B <-- A",
    ])
    .unwrap();
    assert_eq!(
        links(&diagram),
        ["lnk5 B->A 2", "lnk7 B->A 1", "lnk9 B->A 1",]
    );
    let cuca = &diagram.cuca;
    let decorated = cuca.link(cuca.get_link_ids()[1]).get_type();
    assert_eq!(decorated.get_decor1(), LinkDecor::CircleCross);
    assert_eq!(decorated.get_decor2(), LinkDecor::ArrowAndCircle);
    let blue = cuca.entity(cuca.get_group("Blue").unwrap());
    assert!(blue.colors.get(ColorType::Line).is_some());
    assert!(blue.colors.get_specific_line_stroke().is_some());
}

#[test]
fn hide_empty_description_is_remembered() {
    let diagram = parse(&["hide empty description", "[*] --> A"]).unwrap();
    assert!(diagram.hide_empty_description);
}
