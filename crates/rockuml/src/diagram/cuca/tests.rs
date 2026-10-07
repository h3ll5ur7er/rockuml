use super::*;
use crate::abel::{EntityGender, LinkArrow, Position};
use crate::color::{ColorType, Colors, HColor};
use crate::decoration::LinkDecor;
use crate::diagram::UmlSource;
use crate::svek::AbstractEntityImage;

fn diagram(separator: Option<&str>) -> CucaDiagram {
    let source = UmlSource::new(Vec::new(), Vec::new());
    let mut diagram = CucaDiagram::new(Titled::new(SName::ClassDiagram, "CLASS", &source));
    diagram.set_namespace_separator(separator);
    diagram
}

/// Declares a leaf the way commands do: its quark resolved in the current group.
fn leaf(diagram: &mut CucaDiagram, name: &str, leaf_type: LeafType) -> EntityId {
    let quark = diagram.quark_in_context(false, name);
    diagram.really_create_leaf(None, quark, Display::create([name]), leaf_type)
}

fn class(diagram: &mut CucaDiagram, name: &str) -> EntityId {
    leaf(diagram, name, LeafType::Class)
}

fn link(diagram: &mut CucaDiagram, from: EntityId, to: EntityId) -> LinkId {
    let link_type = LinkType::new(LinkDecor::None, LinkDecor::Arrow);
    let link = diagram.new_link(None, from, to, link_type, LinkArg::no_display(2));
    diagram.add_link(link);
    link
}

fn enter_package(diagram: &mut CucaDiagram, name: &str) -> EntityId {
    let quark = diagram.quark_in_context(false, name);
    diagram.goto_group(None, quark, Display::create([name]), GroupType::Package);
    diagram.get_current_group()
}

fn qualified(diagram: &CucaDiagram, quark: QuarkId) -> &str {
    diagram.quark(quark).get_qualified_name()
}

fn uid(diagram: &CucaDiagram, entity: EntityId) -> &str {
    diagram.entity(entity).get_uid()
}

fn uids(diagram: &CucaDiagram, entities: &[EntityId]) -> Vec<String> {
    entities
        .iter()
        .map(|entity| uid(diagram, *entity).to_owned())
        .collect()
}

#[test]
fn without_separator_a_name_is_found_anywhere() {
    let mut diagram = diagram(None);
    let dotted = diagram.quark_in_context_safe(true, "a.b").unwrap();
    assert_eq!(diagram.quark(dotted).get_name(), "a.b");
    assert_eq!(
        diagram.quark(dotted).get_parent(),
        Some(diagram.quarks().next().unwrap())
    );
    enter_package(&mut diagram, "P");
    let inner = diagram.quark_in_context_safe(false, "x").unwrap();
    assert_eq!(qualified(&diagram, inner), "P\u{1}x");
    diagram.end_group();
    assert_eq!(diagram.quark_in_context_safe(false, "x"), Ok(inner));
    assert_eq!(diagram.quark_in_context_safe(false, "a.b"), Ok(dotted));
}

#[test]
fn with_a_separator_names_resolve_from_the_current_group_or_the_root() {
    let mut diagram = diagram(Some("."));
    let deep = diagram.quark_in_context_safe(false, "a.b.C").unwrap();
    assert_eq!(qualified(&diagram, deep), "a.b.C");
    enter_package(&mut diagram, "a");
    let relative = diagram.quark_in_context_safe(false, "C2").unwrap();
    assert_eq!(qualified(&diagram, relative), "a.C2");
    let absolute = diagram.quark_in_context_safe(false, ".D").unwrap();
    assert_eq!(qualified(&diagram, absolute), "D");
    let known_root_package = diagram.quark_in_context_safe(false, "a.b.E").unwrap();
    assert_eq!(qualified(&diagram, known_root_package), "a.b.E");
    let unknown_package = diagram.quark_in_context_safe(false, "z.Y").unwrap();
    assert_eq!(qualified(&diagram, unknown_package), "a.z.Y");
}

#[test]
fn a_name_used_once_elsewhere_is_reused_when_asked() {
    let mut diagram = diagram(Some("."));
    let elsewhere = diagram.quark_in_context_safe(false, "q.Target").unwrap();
    enter_package(&mut diagram, "p");
    assert_eq!(diagram.quark_in_context_safe(true, "Target"), Ok(elsewhere));
    let own = diagram.quark_in_context_safe(false, "Target").unwrap();
    assert_eq!(qualified(&diagram, own), "p.Target");
    // Now that two quarks bear the name, none is reused.
    let quark = diagram.quark_in_context_safe(true, "Target").unwrap();
    assert_eq!(quark, own);
}

#[test]
fn double_colons_separate_like_dots() {
    let mut diagram = diagram(Some("::"));
    let quark = diagram
        .quark_in_context_safe(false, "ns::sub::Klass")
        .unwrap();
    assert_eq!(qualified(&diagram, quark), "ns::sub::Klass");
    assert_eq!(diagram.remove_port_id("A::port"), "A::port");
    assert_eq!(diagram.get_port_id("A::port"), None);
    let dotted = self::diagram(Some("."));
    assert_eq!(dotted.remove_port_id("A::port"), "A");
    assert_eq!(dotted.get_port_id("A::port"), Some("port"));
}

#[test]
fn bad_names_and_leaves_used_as_packages_are_errors() {
    let mut diagram = diagram(Some("."));
    let bad = |error: &str, score| {
        Err(Failure {
            error: error.to_owned(),
            score,
        })
    };
    assert_eq!(
        diagram.quark_in_context_safe(false, "a."),
        bad("Bad name since . is a separator", 3)
    );
    assert_eq!(
        diagram.quark_in_context_safe(false, "a..b"),
        bad("Bad name since . is a separator", 3)
    );
    class(&mut diagram, "Foo");
    assert_eq!(
        diagram.quark_in_context_safe(false, "Foo.bar"),
        bad("Not a package: Foo", 0)
    );
}

#[test]
fn names_lose_quotes_and_brackets() {
    assert_eq!(CucaDiagram::clean_id("\"A B\""), "A B");
    assert_eq!(CucaDiagram::clean_id("(use)"), "use");
    assert_eq!(CucaDiagram::clean_id("[comp]"), "comp");
    assert_eq!(CucaDiagram::clean_id(":actor:"), "actor");
    assert_eq!(CucaDiagram::clean_id(":"), ":");
    assert_eq!(CucaDiagram::clean_id("plain"), "plain");
}

#[test]
fn entities_links_and_names_share_one_counter() {
    let mut diagram = diagram(Some("."));
    assert_eq!(uid(&diagram, diagram.get_root_group()), "entroot");
    let a = class(&mut diagram, "A");
    let b = class(&mut diagram, "B");
    let ab = link(&mut diagram, a, b);
    let ba = diagram.get_inv(ab);
    let note_name = diagram.get_unique_sequence("GMN");
    let note = leaf(&mut diagram, &note_name, LeafType::Note);
    let c = class(&mut diagram, "p.q.C");
    assert_eq!(uids(&diagram, &[a, b]), ["ent0001", "ent0002"]);
    assert_eq!(diagram.link(ab).get_uid(), "lnk3");
    assert_eq!(diagram.link(ba).get_uid(), "lnk4");
    assert_eq!(note_name, "GMN5");
    assert_eq!(uid(&diagram, note), "ent0006");
    assert_eq!(uid(&diagram, c), "ent0007");
    let phantoms = diagram.groups();
    assert_eq!(uids(&diagram, &phantoms), ["ent0008", "ent0009"]);
    assert_eq!(diagram.get_unique_sequence_value(), 10);
}

#[test]
fn the_second_counter_restarts_with_every_pass() {
    let mut diagram = diagram(Some("."));
    assert_eq!(diagram.get_unique_sequence2("CONC"), "CONC1");
    assert_eq!(diagram.get_unique_sequence2("CONC"), "CONC2");
    let a = class(&mut diagram, "A");
    enter_package(&mut diagram, "p");
    diagram.goto_together();
    diagram.starting_pass();
    assert_eq!(diagram.get_unique_sequence2("CONC"), "CONC1");
    assert_eq!(diagram.get_last_entity(), None);
    assert_eq!(diagram.get_current_group(), diagram.get_root_group());
    assert_eq!(diagram.current_together(), None);
    assert_eq!(uid(&diagram, a), "ent0001");
    assert_eq!(diagram.get_unique_sequence("x"), "x3");
}

#[test]
fn class_like_leaves_turn_the_names_above_them_into_packages() {
    let mut diagram = diagram(Some("."));
    let c = class(&mut diagram, "p.q.C");
    let groups = diagram.groups();
    assert_eq!(groups.len(), 2);
    let (p, q) = (diagram.entity(groups[0]), diagram.entity(groups[1]));
    assert_eq!(p.get_group_type(), GroupType::Package);
    assert_eq!(p.display.lines(), ["p"]);
    assert_eq!(q.display.lines(), ["q"]);
    assert_eq!(
        diagram.entity(c).get_parent_container(&diagram),
        Some(groups[1])
    );
    assert_eq!(q.get_parent_container(&diagram), Some(groups[0]));
    assert_eq!(p.groups(&diagram), [groups[1]]);
    assert_eq!(q.leafs(&diagram), [c]);
    // Other leaves leave their names alone until the diagram is drawn.
    let mut description = self::diagram(Some("."));
    leaf(&mut description, "r.Usecase", LeafType::Usecase);
    assert_eq!(description.groups().len(), 0);
    description.eventually_build_phantom_groups(None);
    assert_eq!(description.groups().len(), 1);
}

#[test]
fn groups_are_entered_left_and_listed_in_name_order() {
    let mut diagram = diagram(Some("."));
    let p = enter_package(&mut diagram, "p");
    let x = class(&mut diagram, "X");
    diagram.goto_together();
    let y = class(&mut diagram, "Y");
    diagram.end_group();
    diagram.end_group();
    let z = class(&mut diagram, "Z");
    assert_eq!(qualified(&diagram, diagram.entity(x).get_quark()), "p.X");
    assert_eq!(diagram.entity(x).together, None);
    let together = diagram
        .entity(y)
        .together
        .expect("Y is in a together block");
    assert_eq!(diagram.get_together(together).parent, None);
    assert_eq!(diagram.leafs(), [x, y, z]);
    assert_eq!(diagram.groups(), [p]);
    assert_eq!(diagram.groups_and_root(), [diagram.get_root_group(), p]);
    assert_eq!(diagram.get_last_entity(), Some(z));
    assert!(diagram.is_group("p"));
    assert_eq!(diagram.get_group("X"), Some(x));
    assert!(!diagram.is_group("X"));
}

#[test]
fn reentering_a_group_keeps_it_and_its_type() {
    let mut diagram = diagram(Some("."));
    let first = enter_package(&mut diagram, "s");
    diagram.end_group();
    let quark = diagram.quark_in_context(false, "s");
    diagram.goto_group(None, quark, Display::create(["other"]), GroupType::State);
    assert_eq!(diagram.get_current_group(), first);
    assert_eq!(diagram.entity(first).get_group_type(), GroupType::State);
    assert_eq!(diagram.entity(first).display.lines(), ["s"]);
}

#[test]
fn single_links_between_linked_entities_are_dropped() {
    let mut diagram = diagram(None);
    let a = class(&mut diagram, "A");
    let b = class(&mut diagram, "B");
    let first = link(&mut diagram, a, b);
    let link_type = LinkType::new(LinkDecor::None, LinkDecor::None);
    let single = diagram.new_link(None, b, a, link_type, LinkArg::no_display(1));
    diagram.link_mut(single).apply_style(Some("single"));
    diagram.add_link(single);
    let again = link(&mut diagram, a, b);
    assert_eq!(diagram.get_link_ids(), [first, again]);
    assert_eq!(diagram.link(again).get_uid(), "lnk5");
    diagram.remove_link(first);
    assert_eq!(diagram.get_link_ids(), [again]);
}

#[test]
fn inverted_links_swap_their_ends() {
    let mut diagram = diagram(None);
    let a = class(&mut diagram, "A");
    let b = class(&mut diagram, "B");
    let link_type = LinkType::new(LinkDecor::Composition, LinkDecor::Arrow);
    let arg = LinkArg::build(Some(Display::create(["uses"])), 2)
        .with_quantifier(Some("1".to_owned()), Some("*".to_owned()));
    let link = diagram.new_link(None, a, b, link_type, arg);
    diagram.link_mut(link).link_arrow = LinkArrow::DirectNormal;
    diagram.set_port_members(link, Some("p1".to_owned()), None);
    let inv = diagram.get_inv(link);
    let inv = diagram.link(inv);
    assert_eq!((inv.get_entity1(), inv.get_entity2()), (b, a));
    assert_eq!(inv.get_type().get_decor1(), LinkDecor::Arrow);
    assert_eq!(inv.get_type().get_decor2(), LinkDecor::Composition);
    assert_eq!(
        (inv.get_quantifier1(), inv.get_quantifier2()),
        (Some("*"), Some("1"))
    );
    assert_eq!(inv.get_label().unwrap().lines(), ["uses"]);
    assert!(inv.is_inverted());
    assert_eq!(inv.get_link_arrow(), LinkArrow::Backward);
    assert_eq!(
        (inv.get_port_name1(), inv.get_port_name2()),
        (None, Some("p1"))
    );
    assert_eq!(
        diagram.entity(a).get_port_short_names().collect::<Vec<_>>(),
        ["p1"]
    );
}

#[test]
fn arrow_styles_set_colours_lines_and_flags() {
    let mut diagram = diagram(None);
    let a = class(&mut diagram, "A");
    let b = class(&mut diagram, "B");
    let id = link(&mut diagram, a, b);
    diagram
        .link_mut(id)
        .apply_style(Some("#red,dashed;#blue,norank"));
    let styled = diagram.link(id);
    assert_eq!(
        styled.get_specific_color(),
        Some(&HColor::parse("red").unwrap().unwrap())
    );
    assert_eq!(
        styled.get_supplementary_colors()[0].get(ColorType::Line),
        Some(&HColor::parse("blue").unwrap().unwrap())
    );
    assert_eq!(
        styled.get_type().get_stroke3(None).to_string(),
        "7.0-7.0-1.0"
    );
    assert!(!styled.constraint);
    assert!(!styled.is_hidden(&diagram));
    diagram.link_mut(id).apply_style(Some("hidden,thickness=3"));
    let styled = diagram.link(id);
    assert_eq!(styled.get_type().get_stroke3(None).thickness, 3.0);
    assert!(styled.is_hidden(&diagram));
}

#[test]
fn the_last_links_skip_notes() {
    let mut diagram = diagram(None);
    let a = class(&mut diagram, "A");
    let b = class(&mut diagram, "B");
    let note = leaf(&mut diagram, "N", LeafType::Note);
    assert_eq!(diagram.get_last_link(), None);
    let first = link(&mut diagram, a, b);
    assert_eq!(diagram.get_two_last_links(), None);
    let second = link(&mut diagram, b, a);
    link(&mut diagram, note, a);
    assert_eq!(diagram.get_last_link(), Some(second));
    assert_eq!(diagram.get_two_last_links(), Some([second, first]));
    assert!(!diagram.is_standalone(note));
}

#[test]
fn hide_and_show_by_name_stereotype_and_tag_the_last_one_winning() {
    let mut diagram = diagram(Some("."));
    let foo = class(&mut diagram, "Foo");
    let bar = class(&mut diagram, "Bar");
    let tagged = class(&mut diagram, "Tagged");
    diagram.entity_mut(bar).stereotype = Some(Stereotype::new("<<Big>>"));
    diagram
        .entity_mut(tagged)
        .add_stereotag(crate::stereo::Stereotag {
            name: "gen".to_owned(),
        });
    diagram.hide_or_show2("Fo*", false);
    diagram.hide_or_show2("<<Big>>", false);
    diagram.hide_or_show2("$gen", false);
    assert!(diagram.is_hidden(foo) && diagram.is_hidden(bar) && diagram.is_hidden(tagged));
    diagram.hide_or_show2("Foo", true);
    assert!(!diagram.entity(foo).is_hidden(&diagram));
    assert!(!diagram.is_removed(foo));
}

#[test]
fn hiding_a_package_hides_what_is_in_it() {
    let mut diagram = diagram(Some("."));
    let p = enter_package(&mut diagram, "p");
    let inner = class(&mut diagram, "Inner");
    diagram.hide_or_show2("Inner2", false);
    diagram.end_group();
    let outer = class(&mut diagram, "Inner2");
    diagram.hide_or_show2("p", false);
    assert!(diagram.entity(inner).is_hidden(&diagram));
    assert!(!diagram.is_hidden(inner));
    assert!(diagram.is_hidden(p));
    // Names written inside a group are relative to it.
    assert!(!diagram.is_hidden(outer));
}

#[test]
fn a_note_on_a_single_entity_shares_its_fate() {
    let mut diagram = diagram(None);
    let a = class(&mut diagram, "A");
    let b = class(&mut diagram, "B");
    let note = leaf(&mut diagram, "N", LeafType::Note);
    let dashed = LinkType::new(LinkDecor::None, LinkDecor::None);
    let on_a = diagram.new_link(None, note, a, dashed, LinkArg::no_display(1));
    diagram.add_link(on_a);
    diagram.remove_or_restore("A", false);
    diagram.hide_or_show2("A", false);
    assert!(diagram.is_removed(note) && diagram.is_hidden(note));
    let on_b = diagram.new_link(None, note, b, dashed, LinkArg::no_display(1));
    diagram.add_link(on_b);
    assert!(!diagram.is_removed(note) && !diagram.is_hidden(note));
}

#[test]
fn removing_unlinked_entities_keeps_linked_ones() {
    let mut diagram = diagram(None);
    let a = class(&mut diagram, "A");
    let b = class(&mut diagram, "B");
    let alone = class(&mut diagram, "Alone");
    let invisible = class(&mut diagram, "Invisible");
    link(&mut diagram, a, b);
    let hidden_type = LinkType::new(LinkDecor::None, LinkDecor::None).get_invisible();
    let hidden = diagram.new_link(None, invisible, a, hidden_type, LinkArg::no_display(1));
    diagram.add_link(hidden);
    diagram.remove_or_restore("@unlinked", false);
    assert!(!diagram.is_removed(a) && !diagram.is_removed(b));
    assert!(diagram.is_removed(alone) && diagram.is_removed(invisible));
    assert!(!diagram.is_removed_ignore_unlinked(alone));
    diagram.remove_or_restore("<<Old>>", false);
    assert!(diagram.is_stereotype_removed(&Stereotype::new("<<Old>>")));
    diagram.link_mut(hidden).stereotype = Some(Stereotype::new("<<Old>>"));
    assert!(diagram.link(hidden).is_removed(&diagram));
}

#[test]
fn portions_show_unless_the_last_matching_command_hides_them() {
    let mut diagram = diagram(None);
    let a = class(&mut diagram, "A");
    let i = leaf(&mut diagram, "I", LeafType::Interface);
    diagram.hide_or_show(&EntityGender::All, EntityPortion::Member, false);
    diagram.hide_or_show(
        &EntityGender::ByEntityType(LeafType::Interface),
        EntityPortion::Method,
        true,
    );
    assert!(!diagram.show_portion(EntityPortion::Field, a));
    assert!(!diagram.show_portion(EntityPortion::Method, a));
    assert!(diagram.show_portion(EntityPortion::Method, i));
    assert!(diagram.show_portion(EntityPortion::CircledCharacter, a));
    let mut strict = self::diagram(None);
    strict.titled.skin.set_param("style", "strictuml");
    let a = class(&mut strict, "A");
    assert!(!strict.show_portion(EntityPortion::CircledCharacter, a));
}

#[test]
fn stereotype_labels_hide_by_name() {
    let mut diagram = diagram(None);
    let a = class(&mut diagram, "A");
    assert_eq!(diagram.get_visible_stereotype_labels(a), None);
    diagram.entity_mut(a).stereotype = Some(Stereotype::new("<<Big>><<Old>>"));
    diagram.hide_or_show(
        &EntityGender::ByStereotype("<<Old>>".to_owned()),
        EntityPortion::Stereotype,
        false,
    );
    assert_eq!(
        diagram.get_visible_stereotype_labels(a),
        Some(vec!["<<Big>>".to_owned()])
    );
}

#[test]
fn three_or_more_standalone_leaves_are_put_in_a_square() {
    let mut diagram = diagram(Some("."));
    let leaves: Vec<EntityId> = ["A", "B", "C", "D", "E"]
        .iter()
        .map(|name| class(&mut diagram, name))
        .collect();
    enter_package(&mut diagram, "p");
    let x = class(&mut diagram, "X");
    let y = class(&mut diagram, "Y");
    diagram.end_group();
    link(&mut diagram, x, y);
    diagram.apply_single_strategy();
    let links: Vec<(EntityId, EntityId, i32, bool, String)> = diagram
        .get_links()
        .skip(1)
        .map(|link| {
            (
                link.get_entity1(),
                link.get_entity2(),
                link.get_length(),
                link.is_invis(),
                link.get_uid().to_owned(),
            )
        })
        .collect();
    let (a, b, c, d, e) = (leaves[0], leaves[1], leaves[2], leaves[3], leaves[4]);
    assert_eq!(
        links,
        [
            (a, b, 1, true, "lnk10".to_owned()),
            (b, c, 1, true, "lnk11".to_owned()),
            (a, d, 2, true, "lnk12".to_owned()),
            (d, e, 1, true, "lnk13".to_owned()),
        ]
    );
}

#[test]
fn squares_of_subgroups_are_put_in_a_square_too() {
    let mut diagram = diagram(Some("."));
    let outer = enter_package(&mut diagram, "outer");
    let mut corners = Vec::new();
    for package in ["p1", "p2", "p3"] {
        enter_package(&mut diagram, package);
        let first = class(&mut diagram, &format!("{package}a"));
        class(&mut diagram, &format!("{package}b"));
        class(&mut diagram, &format!("{package}c"));
        corners.push(first);
        diagram.end_group();
    }
    diagram.end_group();
    diagram.apply_single_strategy();
    let between_squares: Vec<(EntityId, EntityId)> = diagram
        .get_links()
        .skip(6)
        .map(|link| (link.get_entity1(), link.get_entity2()))
        .collect();
    // In a square of three, the second leaf ends the first row and the third starts the last.
    let top_right = |first: EntityId| EntityId(first.0 + 1);
    let bottom_left = |first: EntityId| EntityId(first.0 + 2);
    assert_eq!(
        between_squares,
        [
            (top_right(corners[0]), corners[1]),
            (bottom_left(corners[0]), corners[2])
        ]
    );
    assert_eq!(diagram.entity(outer).groups(&diagram).len(), 3);
}

#[test]
fn packages_holding_only_a_package_are_shown_as_one() {
    let mut diagram = diagram(Some("."));
    let a = enter_package(&mut diagram, "a");
    let b = enter_package(&mut diagram, "b");
    let c = enter_package(&mut diagram, "c");
    class(&mut diagram, "C");
    diagram.end_group();
    diagram.end_group();
    diagram.end_group();
    let lonely = enter_package(&mut diagram, "lonely");
    let inside = enter_package(&mut diagram, "inside");
    diagram.end_group();
    diagram.end_group();
    diagram.pack_some_package();
    assert!(diagram.entity(a).is_packed() && diagram.entity(b).is_packed());
    assert!(!diagram.entity(c).is_packed());
    assert_eq!(diagram.entity(c).display.lines(), ["a.b.c"]);
    assert!(!diagram.entity(lonely).is_packed());
    assert_eq!(diagram.entity(inside).display.lines(), ["inside"]);
}

#[test]
fn linked_packages_are_not_packed() {
    let mut diagram = diagram(Some("."));
    let a = enter_package(&mut diagram, "a");
    enter_package(&mut diagram, "b");
    let c = class(&mut diagram, "C");
    diagram.end_group();
    diagram.end_group();
    link(&mut diagram, a, c);
    assert!(!diagram.entity(a).can_be_packed(&diagram));
}

#[test]
fn groups_whose_links_stay_inside_or_outside_are_autarkic() {
    let mut diagram = diagram(Some("."));
    let s = enter_package(&mut diagram, "s");
    diagram.entity_mut(s).mute_to_group_type(GroupType::State);
    let inner1 = leaf(&mut diagram, "I1", LeafType::State);
    let inner2 = leaf(&mut diagram, "I2", LeafType::State);
    diagram.end_group();
    let outer = leaf(&mut diagram, "O", LeafType::State);
    let inside = link(&mut diagram, inner1, inner2);
    assert!(diagram.entity(s).is_autarkic(&diagram));
    let crossing = link(&mut diagram, outer, inner1);
    assert!(!diagram.entity(s).is_autarkic(&diagram));
    let group = diagram.entity(s);
    assert!(is_pure_inner_link12(group, diagram.link(inside), &diagram));
    assert!(!is_pure_inner_link12(
        group,
        diagram.link(crossing),
        &diagram
    ));
    diagram.remove_link(crossing);
    diagram.entity_mut(inner2).stereotype = Some(Stereotype::new("<<exitPoint>>"));
    assert!(!diagram.entity(s).is_autarkic(&diagram));
    assert!(diagram.link(inside).has_entry_point(&diagram));
}

#[test]
fn a_group_laid_out_alone_becomes_a_leaf_without_its_inner_links() {
    let mut diagram = diagram(Some("."));
    let s = enter_package(&mut diagram, "s");
    let inner1 = class(&mut diagram, "I1");
    let inner2 = class(&mut diagram, "I2");
    diagram.end_group();
    let outer = class(&mut diagram, "O");
    link(&mut diagram, inner1, inner2);
    let crossing = link(&mut diagram, outer, s);
    diagram.override_image(s, LeafType::State);
    assert_eq!(diagram.get_link_ids(), [crossing]);
    assert_eq!(diagram.entity(s).get_leaf_type(), Some(LeafType::State));
}

#[test]
fn types_mute_between_class_like_ones_only() {
    let mut diagram = diagram(None);
    let a = class(&mut diagram, "A");
    let entity = diagram.entity_mut(a);
    assert!(entity.mute_to_type_if_compatible(LeafType::Object));
    assert_eq!(entity.get_leaf_type(), Some(LeafType::Object));
    assert!(!entity.mute_to_type_if_compatible(LeafType::Interface));
    entity.mute_to_type(LeafType::StillUnknown);
    assert!(entity.mute_to_type_if_compatible(LeafType::Usecase));
    assert!(entity.mute_to_type_if_compatible(LeafType::Usecase));
    entity.mute_to_group_type(GroupType::Package);
    assert!(entity.is_group() && entity.get_leaf_type().is_none());
}

#[test]
fn notes_and_tips_hang_on_entities() {
    let mut diagram = diagram(None);
    let a = class(&mut diagram, "A");
    let entity = diagram.entity_mut(a);
    entity.add_note(Display::create(["top"]), Position::Top, Colors::default());
    entity.add_note(Display::create(["left"]), Position::Left, Colors::default());
    entity.put_tip(
        "m".to_owned(),
        Display::create(["1"]),
        Colors::default(),
        None,
    );
    entity.put_tip(
        "n".to_owned(),
        Display::create(["2"]),
        Colors::default(),
        None,
    );
    entity.put_tip(
        "m".to_owned(),
        Display::create(["3"]),
        Colors::default(),
        None,
    );
    assert_eq!(entity.get_notes(Position::Top).len(), 1);
    assert_eq!(entity.get_notes(Position::Bottom), []);
    let tips: Vec<(&str, &[String])> = entity
        .get_tips()
        .iter()
        .map(|(member, tip)| (member.as_str(), tip.display.lines()))
        .collect();
    assert_eq!(
        tips,
        [("m", &["3".to_owned()][..]), ("n", &["2".to_owned()][..])]
    );
}

#[test]
fn states_on_the_border_take_their_position_from_the_stereotype() {
    let mut diagram = diagram(None);
    let state = leaf(&mut diagram, "S", LeafType::State);
    let class = class(&mut diagram, "C");
    diagram.entity_mut(state).stereotype = Some(Stereotype::new("<<entryPoint>>"));
    diagram.entity_mut(class).stereotype = Some(Stereotype::new("<<entryPoint>>"));
    assert_eq!(
        diagram.entity(state).get_entity_position(),
        crate::abel::EntityPosition::EntryPoint
    );
    assert_eq!(
        diagram.entity(class).get_entity_position(),
        crate::abel::EntityPosition::Normal
    );
}

#[test]
fn entity_images_read_what_they_need_of_the_entity() {
    let mut diagram = diagram(None);
    let a = class(&mut diagram, "A");
    diagram.hide_or_show2("A", false);
    diagram.titled.skin.set_param("backgroundColor", "#EEEEEE");
    let image = AbstractEntityImage::new(diagram.entity(a), &diagram);
    assert!(image.is_hidden());
    assert_eq!(image.get_entity(), a);
    assert_eq!(
        image.get_backcolor(),
        HColor::parse("#EEEEEE").unwrap().unwrap()
    );
    assert_eq!(image.get_style_name(), SName::ClassDiagram);
}

#[test]
fn the_default_margins_are_plantumls() {
    let margins = CucaDiagram::get_default_margins();
    assert_eq!(
        (margins.top, margins.right, margins.bottom, margins.left),
        (0.0, 5.0, 5.0, 0.0)
    );
}

fn class_diagram() -> AbstractClassOrObjectDiagram {
    let source = UmlSource::new(Vec::new(), Vec::new());
    AbstractClassOrObjectDiagram::new(Titled::new(SName::ClassDiagram, "CLASS", &source))
}

#[test]
fn association_classes_cut_the_link_at_a_point() {
    let mut class_diagram = class_diagram();
    let diagram = &mut class_diagram.cuca;
    assert_eq!(diagram.get_namespace_separator(), Some("."));
    let a = class(diagram, "A");
    let b = class(diagram, "B");
    let ab = diagram.new_link(
        None,
        a,
        b,
        LinkType::new(LinkDecor::Aggregation, LinkDecor::Arrow),
        LinkArg::no_display(1),
    );
    diagram.add_link(ab);
    let c = class(diagram, "C");
    let d = class(diagram, "D");
    let dotted = LinkType::new(LinkDecor::None, LinkDecor::None).go_dotted();
    assert!(class_diagram.association_class(None, 1, a, b, c, dotted, None));
    assert!(class_diagram.association_class(None, 1, a, b, d, dotted, None));
    assert!(!class_diagram.association_class(None, 1, a, b, c, dotted, None));
    let diagram = &class_diagram.cuca;
    let points: Vec<EntityId> = diagram
        .leafs()
        .into_iter()
        .filter(|leaf| diagram.entity(*leaf).get_leaf_type() == Some(LeafType::PointForAssociation))
        .collect();
    assert_eq!(uids(diagram, &points), ["ent0007", "ent0012"]);
    assert_eq!(diagram.entity(points[0]).get_name(diagram), "apoint6");
    let (p1, p2) = (points[0], points[1]);
    let links: Vec<(String, EntityId, EntityId, i32)> = diagram
        .get_links()
        .map(|link| {
            (
                link.get_uid().to_owned(),
                link.get_entity1(),
                link.get_entity2(),
                link.get_length(),
            )
        })
        .collect();
    let expected = [
        ("lnk8", a, p1, 2),
        ("lnk9", p1, b, 2),
        ("lnk14", a, p2, 2),
        ("lnk15", p2, b, 2),
        ("lnk16", c, p1, 1),
        ("lnk17", p2, d, 1),
        ("lnk18", p1, p2, 1),
    ]
    .map(|(uid, from, to, length)| (uid.to_owned(), from, to, length));
    assert_eq!(links, expected);
    let mut halves = diagram.get_links();
    let (first, second) = (halves.next().unwrap(), halves.next().unwrap());
    assert_eq!(first.get_type().get_decor1(), LinkDecor::None);
    assert_eq!(first.get_type().get_decor2(), LinkDecor::Arrow);
    assert_eq!(second.get_type().get_decor1(), LinkDecor::Aggregation);
    assert!(diagram.get_links().last().unwrap().is_invis());
}

#[test]
fn links_are_cut_at_a_node_inserted_between() {
    let mut class_diagram = class_diagram();
    let diagram = &mut class_diagram.cuca;
    let a = class(diagram, "A");
    let b = class(diagram, "B");
    let node = leaf(diagram, "N", LeafType::Association);
    let arg = LinkArg::build(Some(Display::create(["l"])), 2)
        .with_quantifier(Some("1".to_owned()), Some("2".to_owned()));
    let link_type = LinkType::new(LinkDecor::None, LinkDecor::Arrow);
    let ab = diagram.new_link(None, a, b, link_type, arg);
    diagram.add_link(ab);
    assert!(class_diagram.insert_between(None, b, a, node));
    assert!(!class_diagram.insert_between(None, a, b, node));
    let diagram = &class_diagram.cuca;
    let halves: Vec<(EntityId, EntityId, Option<&str>, Option<&str>)> = diagram
        .get_links()
        .map(|link| {
            (
                link.get_entity1(),
                link.get_entity2(),
                link.get_quantifier1(),
                link.get_quantifier2(),
            )
        })
        .collect();
    assert_eq!(
        halves,
        [(b, node, Some("1"), None), (node, a, None, Some("2"))]
    );
}
