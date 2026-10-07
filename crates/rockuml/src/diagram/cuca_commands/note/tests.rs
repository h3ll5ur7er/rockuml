use super::*;
use crate::abel::LinkId;
use crate::command::{CommandControl, CommandError};
use crate::diagram::UmlSource;
use crate::diagram::titled::Titled;
use crate::style::SName;

struct Diagram(CucaDiagram);

impl EntityDiagram for Diagram {
    fn cuca(&mut self) -> &mut CucaDiagram {
        &mut self.0
    }
}

fn diagram() -> Diagram {
    let source = UmlSource::new(Vec::new(), Vec::new());
    Diagram(CucaDiagram::new(Titled::new(
        SName::ClassDiagram,
        "CLASS",
        &source,
    )))
}

fn code() -> RegexTree {
    RegexTree::named(1, "CODE", "([^%s{}%g<>]+|[%g][^%g]+[%g])")
}

fn class(diagram: &mut Diagram, name: &str) -> EntityId {
    let cuca = diagram.cuca();
    let quark = cuca.quark_in_context(false, name).unwrap();
    cuca.really_create_leaf(None, quark, Display::create([name]), LeafType::Class)
}

fn link(diagram: &mut Diagram, from: EntityId, to: EntityId) -> LinkId {
    let cuca = diagram.cuca();
    let link_type = LinkType::new(LinkDecor::None, LinkDecor::Arrow);
    let link = cuca.new_link(None, from, to, link_type, LinkArg::no_display(2));
    cuca.add_link(link);
    link
}

fn run(command: &dyn Command<Diagram>, diagram: &mut Diagram, lines: &[&str]) -> CommandResult {
    let lines = BlocLines::from_texts(lines);
    assert_eq!(command.is_valid(&lines), CommandControl::Ok, "{lines:?}");
    command.execute(diagram, lines)
}

fn named(diagram: &mut Diagram, name: &str) -> EntityId {
    let cuca = diagram.cuca();
    let quark = cuca.first_with_name(name).unwrap();
    cuca.quark(quark).get_data().unwrap()
}

fn back(diagram: &mut Diagram, entity: EntityId) -> Option<HColor> {
    diagram
        .cuca()
        .entity(entity)
        .colors
        .get(ColorType::Back)
        .cloned()
}

fn pink() -> HColor {
    HColor::parse("pink").unwrap().unwrap()
}

#[test]
fn a_named_note_is_a_leaf_links_can_reach() {
    let mut diagram = diagram();
    run(
        note().as_ref(),
        &mut diagram,
        &["note \"Shared\\nby both\" as N1 $tag <<s>> #pink"],
    )
    .unwrap();
    let n1 = named(&mut diagram, "N1");
    let cuca = diagram.cuca();
    let entity = cuca.entity(n1);
    assert_eq!(entity.get_leaf_type(), Some(LeafType::Note));
    assert_eq!(entity.display.lines(), ["Shared", "by both"]);
    assert_eq!(entity.stereotype, Some(Stereotype::new("<<s>>")));
    assert_eq!(entity.stereotags()[0].name, "tag");
    assert_eq!(back(&mut diagram, n1), Some(pink()));
    let again = run(note().as_ref(), &mut diagram, &["note \"Other\" as N1"]);
    assert_eq!(again, Err(CommandError::new("Note already created: N1")));
}

#[test]
fn a_block_note_takes_the_lines_in_between_without_their_indentation() {
    let mut diagram = diagram();
    run(
        note_multi_line().as_ref(),
        &mut diagram,
        &[
            "note as Remark",
            "  This note floats",
            "    free",
            "end note",
        ],
    )
    .unwrap();
    let remark = named(&mut diagram, "Remark");
    assert_eq!(
        diagram.cuca().entity(remark).display.lines(),
        ["This note floats", "  free"]
    );
}

#[test]
fn a_note_on_an_entity_is_a_numbered_leaf_with_a_dashed_link() {
    let mut diagram = diagram();
    let server = class(&mut diagram, "Server");
    let command = note_on_entity(code, ParserPass::One);
    run(
        command.as_ref(),
        &mut diagram,
        &["note top of Server : listens"],
    )
    .unwrap();
    run(
        command.as_ref(),
        &mut diagram,
        &["note right of Server #pink : stateless"],
    )
    .unwrap();
    let cuca = diagram.cuca();
    let links: Vec<&crate::abel::Link> = cuca.get_links().collect();
    let [top, right] = links[..] else {
        panic!("two links")
    };
    // The class took 1, the first note's name 2, its entity 3 and its link 4.
    let gmn2 = top.get_entity1();
    assert_eq!(cuca.entity(gmn2).get_name(cuca), "GMN2");
    assert_eq!(cuca.entity(gmn2).get_uid(), "ent0003");
    assert_eq!(top.get_uid(), "lnk4");
    assert_eq!(top.get_entity2(), server);
    assert_eq!(top.get_length(), 2);
    assert!(!top.horizontal_solitary);
    let dashed = LinkType::new(LinkDecor::None, LinkDecor::None).go_dashed();
    assert_eq!(top.get_type(), dashed);
    let gmn5 = right.get_entity2();
    assert_eq!(right.get_entity1(), server);
    assert_eq!(cuca.entity(gmn5).get_name(cuca), "GMN5");
    assert_eq!(cuca.entity(gmn5).display.lines(), ["stateless"]);
    assert_eq!(right.get_length(), 1);
    assert!(right.horizontal_solitary);
    assert_eq!(back(&mut diagram, gmn5), Some(pink()));
}

#[test]
fn left_to_right_diagrams_turn_note_sides_a_quarter() {
    let mut diagram = diagram();
    diagram
        .cuca()
        .titled
        .skin
        .set_rankdir(crate::skin::Rankdir::LeftToRight);
    let a = class(&mut diagram, "A");
    run(
        note_on_entity(code, ParserPass::One).as_ref(),
        &mut diagram,
        &["note right of A : below"],
    )
    .unwrap();
    run(
        tip_on_entity_multi_line(false).as_ref(),
        &mut diagram,
        &["note left of A::m", "above", "end note"],
    )
    .unwrap();
    let cuca = diagram.cuca();
    let links: Vec<&crate::abel::Link> = cuca.get_links().collect();
    assert_eq!(links[0].get_entity1(), a);
    assert_eq!(links[0].get_length(), 2);
    assert_eq!(
        cuca.entity(links[1].get_entity1()).get_name(cuca),
        "A$$$TOP"
    );
}

#[test]
fn a_note_without_target_goes_on_the_latest_entity() {
    let mut diagram = diagram();
    let command = note_on_entity(code, ParserPass::One);
    assert_eq!(
        run(command.as_ref(), &mut diagram, &["note left : x"]),
        Err(CommandError::new("Nothing to note to"))
    );
    let a = class(&mut diagram, "A");
    run(command.as_ref(), &mut diagram, &["note left : x"]).unwrap();
    let link = diagram.cuca().get_links().next().unwrap();
    assert_eq!(link.get_entity2(), a);
    assert_eq!(
        run(command.as_ref(), &mut diagram, &["note left of B : x"]),
        Err(CommandError::new("Not known: B"))
    );
}

#[test]
fn a_block_note_on_an_entity_may_open_with_a_bracket_and_keeps_its_link() {
    let mut diagram = diagram();
    class(&mut diagram, "A");
    let bracketed = note_on_entity_multi_line(code, ParserPass::One, true);
    run(
        bracketed.as_ref(),
        &mut diagram,
        &["note bottom of A [[https://plantuml.com]] {", "  one", "}"],
    )
    .unwrap();
    let note = diagram.cuca().get_links().next().unwrap().get_entity2();
    let entity = diagram.cuca().entity(note);
    assert_eq!(entity.display.lines(), ["one"]);
    assert_eq!(
        entity.url.as_ref().map(|url| url.href.as_str()),
        Some("https://plantuml.com")
    );
    let plain = note_on_entity_multi_line::<Diagram>(code, ParserPass::One, false);
    let lines = BlocLines::from_texts(&["note left of A", "two", "  end note"]);
    assert_eq!(plain.is_valid(&lines), CommandControl::Ok);
}

#[test]
fn notes_on_entities_run_in_one_pass() {
    let command = note_on_entity::<Diagram>(code, ParserPass::Three);
    assert!(!command.is_eligible_for(ParserPass::One));
    assert!(command.is_eligible_for(ParserPass::Three));
}

#[test]
fn a_note_on_a_link_goes_on_the_latest_link_between_entities_not_notes() {
    let mut diagram = diagram();
    let command = note_on_link(ParserPass::One);
    assert_eq!(
        run(command.as_ref(), &mut diagram, &["note on link : x"]),
        Err(CommandError::new("No link defined"))
    );
    let a = class(&mut diagram, "A");
    let b = class(&mut diagram, "B");
    let ab = link(&mut diagram, a, b);
    run(
        note_on_entity(code, ParserPass::One).as_ref(),
        &mut diagram,
        &["note right of A : n"],
    )
    .unwrap();
    run(
        command.as_ref(),
        &mut diagram,
        &["note on link #pink : one"],
    )
    .unwrap();
    let note = diagram.cuca().link(ab).note.clone().unwrap();
    assert_eq!(note.display.lines(), ["one"]);
    assert_eq!(note.position, Position::Bottom);
    assert_eq!(note.colors.get(ColorType::Back), Some(&pink()));
    let multi = note_on_link_multi_line(ParserPass::One);
    run(
        multi.as_ref(),
        &mut diagram,
        &["note right on link", "  two", "end note"],
    )
    .unwrap();
    let note = diagram.cuca().link(ab).note.clone().unwrap();
    assert_eq!(note.display.lines(), ["two"]);
    assert_eq!(note.position, Position::Right);
    assert_eq!(
        run(multi.as_ref(), &mut diagram, &["note on link", "end note"]),
        Err(CommandError::new("No note defined"))
    );
}

#[test]
fn tips_on_one_side_of_an_entity_share_a_leaf_and_keep_their_colours() {
    let mut diagram = diagram();
    let thread = class(&mut diagram, "Thread");
    let command = tip_on_entity_multi_line(false);
    run(
        command.as_ref(),
        &mut diagram,
        &["note right of Thread::start #pink", "spawns", "end note"],
    )
    .unwrap();
    run(
        command.as_ref(),
        &mut diagram,
        &["note right of Thread::\"run()\"", "runs", "end note"],
    )
    .unwrap();
    run(
        tip_on_entity_multi_line(true).as_ref(),
        &mut diagram,
        &["note left of Thread::priority {", "1 to 10", "}"],
    )
    .unwrap();
    let cuca = diagram.cuca();
    let links: Vec<&crate::abel::Link> = cuca.get_links().collect();
    let [right, left] = links[..] else {
        panic!("one link per side")
    };
    assert_eq!(right.get_entity1(), thread);
    assert_eq!(left.get_entity2(), thread);
    assert!(right.get_type().is_invisible());
    let tips = cuca.entity(right.get_entity2());
    assert_eq!(tips.get_name(cuca), "Thread$$$RIGHT");
    assert_eq!(tips.get_leaf_type(), Some(LeafType::Tips));
    let members: Vec<&str> = tips.get_tips().iter().map(|(m, _)| m.as_str()).collect();
    assert_eq!(members, ["start", "run()"]);
    assert_eq!(
        tips.get_tips()[0].1.colors.get(ColorType::Back),
        Some(&pink())
    );
    assert_eq!(tips.get_tips()[1].1.colors.get(ColorType::Back), None);
    assert_eq!(
        cuca.entity(left.get_entity1()).get_name(cuca),
        "Thread$$$LEFT"
    );
}

#[test]
fn a_constraint_needs_two_links() {
    let mut diagram = diagram();
    let command = constraint_on_links();
    let a = class(&mut diagram, "A");
    let b = class(&mut diagram, "B");
    link(&mut diagram, a, b);
    assert_eq!(
        run(
            command.as_ref(),
            &mut diagram,
            &["constraint on links : {xor}"]
        ),
        Err(CommandError::new("Cannot put constraint on two last links"))
    );
    link(&mut diagram, b, a);
    run(
        command.as_ref(),
        &mut diagram,
        &["constraint on links : {xor}"],
    )
    .unwrap();
}
