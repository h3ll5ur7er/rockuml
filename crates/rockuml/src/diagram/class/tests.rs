use super::*;
use crate::abel::{EntityId, LeafType, LinkArrow};
use crate::command::factory::{self, Created};
use crate::decoration::LinkDecor;
use crate::text::{LineLocation, StringLocated};

/// The class diagram the lines make, or the error they get.
fn parse(texts: &[&str]) -> Result<ClassDiagram, String> {
    let location = LineLocation::new("test", None);
    let lines: Vec<StringLocated> = ["@startuml"]
        .iter()
        .chain(texts)
        .chain(&["@enduml"])
        .map(|text| StringLocated::new(*text, location.clone()))
        .collect();
    let source = Rc::new(UmlSource::new(lines, Vec::new()));
    let commands = ClassDiagramFactory::init_commands_list();
    match factory::create_system(
        &source,
        || ClassDiagramFactory::create_empty_diagram(&source),
        &commands,
    ) {
        Created::Diagram(diagram) => Ok(diagram),
        Created::Failure(failure) => Err(failure.error.message),
        Created::Nothing => Err("nothing".to_owned()),
    }
}

fn entity(diagram: &ClassDiagram, name: &str) -> EntityId {
    let cuca = &diagram.diagram.cuca;
    let quark = cuca.first_with_name(name).expect("a known name");
    cuca.quark(quark).get_data().expect("an entity")
}

fn members(diagram: &ClassDiagram, name: &str, fields: bool) -> Vec<String> {
    let cuca = &diagram.diagram.cuca;
    let bodier = cuca.entity(entity(diagram, name)).get_bodier();
    let hidden = cuca.get_hides_visibility_modifier();
    let members = if fields {
        bodier.get_fields_to_display(hidden)
    } else {
        bodier.get_methods_to_display(hidden)
    };
    members
        .iter()
        .map(|member| member.get_display(true))
        .collect()
}

#[test]
fn classes_read_their_members_from_a_block_or_one_by_one() {
    let diagram = parse(&[
        "abstract class Car <<Entity>> #pink {",
        "  - String model",
        "  {static} + count() : int",
        "}",
        "Car : # int year",
        "interface Drivable",
    ])
    .unwrap();
    let cuca = &diagram.diagram.cuca;
    let car = cuca.entity(entity(&diagram, "Car"));
    assert_eq!(car.get_leaf_type(), Some(LeafType::AbstractClass));
    assert_eq!(car.get_uid(), "ent0001");
    assert_eq!(
        car.stereotype.as_ref().unwrap().label_double_comparator(),
        "<<Entity>>"
    );
    assert!(car.colors.get(crate::color::ColorType::Back).is_some());
    assert_eq!(
        members(&diagram, "Car", true),
        ["-String model", "#int year"]
    );
    assert_eq!(members(&diagram, "Car", false), ["+count() : int"]);
    let drivable = cuca.entity(entity(&diagram, "Drivable"));
    assert_eq!(drivable.get_leaf_type(), Some(LeafType::Interface));
}

#[test]
fn links_read_their_decorations_labels_and_direction() {
    let diagram = parse(&[
        "class A",
        r#"A "1" *-- "many" B : contains >"#,
        "A -left-> C",
        "A ..|> D",
    ])
    .unwrap();
    let cuca = &diagram.diagram.cuca;
    let links: Vec<_> = cuca.get_links().collect();
    assert_eq!(links.len(), 3);
    let contains = links[0];
    assert_eq!(contains.get_type().get_decor2(), LinkDecor::Composition);
    assert_eq!(contains.get_quantifier1(), Some("1"));
    assert_eq!(contains.get_quantifier2(), Some("many"));
    assert_eq!(contains.get_label().unwrap().lines(), ["contains"]);
    assert_eq!(contains.get_link_arrow(), LinkArrow::DirectNormal);
    assert_eq!(contains.get_length(), 2);
    let left = links[1];
    assert!(left.is_inverted(), "left arrows are drawn from their end");
    assert_eq!(left.get_entity1(), entity(&diagram, "C"));
    assert_eq!(left.get_length(), 1);
    assert_eq!(left.get_uid(), "lnk6");
    let implements = links[2];
    assert_eq!(implements.get_type().get_decor1(), LinkDecor::Extends);
}

#[test]
fn extends_and_implements_link_to_their_classes() {
    let diagram = parse(&["class Job extends Task implements Runnable"]).unwrap();
    let cuca = &diagram.diagram.cuca;
    let runnable = cuca.entity(entity(&diagram, "Runnable"));
    assert_eq!(runnable.get_leaf_type(), Some(LeafType::Interface));
    let links: Vec<_> = cuca.get_links().collect();
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].get_entity1(), entity(&diagram, "Task"));
    assert_eq!(links[0].get_entity2(), entity(&diagram, "Job"));
}

#[test]
fn objects_maps_and_json_get_their_bodies() {
    let diagram = parse(&[
        "object user {",
        r#"  name = "Dummy""#,
        "}",
        "map CapitalCity {",
        "  UK => London",
        "  USA *-> user",
        "}",
        "json J {",
        r#"  "a": [1, 2]"#,
        "}",
    ])
    .unwrap();
    assert_eq!(members(&diagram, "user", true), [r#"name = "Dummy""#]);
    let cuca = &diagram.diagram.cuca;
    let map_link = cuca.get_links().next().unwrap();
    assert_eq!(map_link.get_port_name1(), Some("USA"));
    assert_eq!(map_link.get_length(), 1);
    assert_eq!(
        cuca.entity(entity(&diagram, "J")).get_leaf_type(),
        Some(LeafType::Json)
    );
}

#[test]
fn packages_and_namespaces_nest_their_classes() {
    let diagram = parse(&[
        "package com.shop {",
        "  class Cart",
        "}",
        "namespace net.dummy {",
        "  class Person",
        "}",
    ])
    .unwrap();
    let cuca = &diagram.diagram.cuca;
    let cart = cuca.entity(entity(&diagram, "Cart"));
    let parent = cart.get_parent_container(cuca).unwrap();
    assert_eq!(cuca.entity(parent).display.lines(), ["shop"]);
    let person = cuca.entity(entity(&diagram, "Person"));
    assert_eq!(
        cuca.quark(person.get_quark()).get_qualified_name(),
        "net.dummy.Person"
    );
}

#[test]
fn hide_commands_select_portions_by_gender() {
    let diagram = parse(&[
        "class Visible {",
        "  a : int",
        "}",
        "class Empty",
        "hide members",
        "show Visible fields",
        "hide empty methods",
        "hide private members",
    ])
    .unwrap();
    let cuca = &diagram.diagram.cuca;
    let visible = entity(&diagram, "Visible");
    let empty = entity(&diagram, "Empty");
    assert!(cuca.show_portion(crate::abel::EntityPortion::Field, visible));
    assert!(!cuca.show_portion(crate::abel::EntityPortion::Method, visible));
    assert!(!cuca.show_portion(crate::abel::EntityPortion::Field, empty));
    assert_eq!(cuca.get_hides_visibility_modifier().len(), 2);
}

#[test]
fn bad_declarations_are_errors() {
    assert_eq!(
        parse(&["class A", "A <<S>>", "B <<S>>"]).err().unwrap(),
        "No such class B"
    );
    assert_eq!(
        parse(&["object a", "object a"]).err().unwrap(),
        "Object already exists: a"
    );
    assert_eq!(
        parse(&["usecase U"]).err().unwrap(),
        "Use 'allowmixing' if you want to mix classes and other UML elements."
    );
}
