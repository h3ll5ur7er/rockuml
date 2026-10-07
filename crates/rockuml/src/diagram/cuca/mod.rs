//! What class, object, description, state and chen diagrams share: entities in a namespace of groups, the
//! links between them, and what `hide`, `show` and `remove` say about them (PlantUML's `net.atmp.CucaDiagram`).
//! Each family's diagram embeds a `CucaDiagram`.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        unused_imports,
        reason = "used by the Phase 5 family commands"
    )
)]
#![cfg_attr(
    test,
    allow(
        dead_code,
        unused_imports,
        reason = "used by the Phase 5 family commands"
    )
)]

mod entity_diagram;
mod hide_or_show;
mod magma;
#[cfg(test)]
mod tests;

pub(crate) use entity_diagram::AbstractClassOrObjectDiagram;

use std::rc::Rc;

use super::titled::Titled;
use crate::abel::{
    Bag, Entity, EntityGender, EntityId, EntityPortion, EntityType, GroupType, LeafType, Link,
    LinkArg, LinkId, Together, TogetherId, is_pure_inner_link12,
};
use crate::creole::Display;
use crate::decoration::LinkType;
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::plasma::{Plasma, Quark, QuarkId};
use crate::skin::SkinParam;
use crate::stereo::Stereotype;
use crate::style::{SName, StyleBuilder};
use crate::text::LineLocation;
use hide_or_show::HideOrShow;

/// A quark name a command cannot use, and how sure PlantUML is that the line was meant for that command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Failure {
    pub error: String,
    pub score: i32,
}

/// `hide` or `show` of a portion of the entities of a gender.
struct EntityHideOrShow {
    gender: EntityGender,
    portion: EntityPortion,
    show: bool,
}

pub(crate) struct CucaDiagram {
    pub(in crate::diagram) titled: Titled,
    namespace_separator: Option<String>,
    namespace: Plasma<EntityId>,
    /// Every entity ever created, in creation order; the root first.
    entities: Vec<Entity>,
    /// Every link ever created, including those never added or since removed.
    links: Vec<Link>,
    /// The diagram's links, in the order they were added.
    link_order: Vec<LinkId>,
    togethers: Vec<Together>,
    /// The groups and `together` blocks the commands are inside, innermost last; the root at the bottom.
    stacks: Vec<Bag>,
    hide_or_shows: Vec<EntityHideOrShow>,
    hides2: Vec<HideOrShow>,
    removed: Vec<HideOrShow>,
    /// Numbers entities, links and other unique names across the whole diagram.
    cpt1: i32,
    /// Numbers names afresh on every parsing pass.
    cpt2: i32,
    raw_layout: i32,
    last_entity: Option<EntityId>,
}

impl CucaDiagram {
    pub(in crate::diagram) fn new(titled: Titled) -> Self {
        let mut diagram = Self {
            titled,
            namespace_separator: None,
            namespace: Plasma::new(),
            entities: Vec::new(),
            links: Vec::new(),
            link_order: Vec::new(),
            togethers: Vec::new(),
            stacks: Vec::new(),
            hide_or_shows: Vec::new(),
            hides2: Vec::new(),
            removed: Vec::new(),
            cpt1: 0,
            cpt2: 0,
            raw_layout: 0,
            last_entity: None,
        };
        let root_entity = diagram.new_entity(
            None,
            QuarkId::ROOT,
            None,
            EntityType::Group(GroupType::Root),
        );
        diagram.stacks.push(Bag::Group(root_entity));
        diagram
    }

    pub(crate) fn skin(&self) -> &SkinParam {
        &self.titled.skin
    }

    /// The diagram's style name, like `classDiagram`, which its elements' styles start from.
    pub(crate) fn get_style_name(&self) -> SName {
        self.titled.diagram_style()
    }

    pub(crate) fn entity(&self, id: EntityId) -> &Entity {
        &self.entities[id.0]
    }

    pub(crate) fn entity_mut(&mut self, id: EntityId) -> &mut Entity {
        &mut self.entities[id.0]
    }

    pub(crate) fn link(&self, id: LinkId) -> &Link {
        &self.links[id.0]
    }

    pub(crate) fn link_mut(&mut self, id: LinkId) -> &mut Link {
        &mut self.links[id.0]
    }

    pub(crate) fn quark(&self, id: QuarkId) -> &Quark<EntityId> {
        self.namespace.quark(id)
    }

    /// The quark `full` names below `parent`, created with any missing parent.
    pub(crate) fn child(&mut self, parent: QuarkId, full: &str) -> QuarkId {
        self.namespace.child(parent, full)
    }

    pub(crate) fn child_if_exists(&self, parent: QuarkId, name: &str) -> Option<QuarkId> {
        self.namespace.child_if_exists(parent, name)
    }

    /// `None` keeps names whole, which `set namespaceSeparator none` asks for.
    pub(crate) fn set_namespace_separator(&mut self, namespace_separator: Option<&str>) {
        self.namespace_separator = namespace_separator.map(str::to_owned);
        self.namespace.set_separator(namespace_separator);
    }

    pub(crate) fn get_namespace_separator(&self) -> Option<&str> {
        self.namespace_separator.as_deref()
    }

    /// A new parsing pass starts at the root, numbering `cpt2` names afresh.
    pub(crate) fn starting_pass(&mut self) {
        self.last_entity = None;
        self.cpt2 = 0;
        self.stacks.truncate(1);
    }

    /// The part after the last `::` of `ent_string`, when it starts with the name of `ident`.
    pub(crate) fn get_port_for(&self, ent_string: &str, ident: QuarkId) -> Option<String> {
        let x = ent_string.rfind("::")?;
        ent_string
            .starts_with(self.quark(ident).get_name())
            .then(|| ent_string[x + 2..].to_owned())
    }

    /// `id` without its `::port`, unless `::` separates namespaces.
    pub(crate) fn remove_port_id<'a>(&self, id: &'a str) -> &'a str {
        if self.get_namespace_separator() == Some("::") {
            return id;
        }
        id.rfind("::").map_or(id, |x| &id[..x])
    }

    /// The `port` of `id::port`, unless `::` separates namespaces.
    pub(crate) fn get_port_id<'a>(&self, id: &'a str) -> Option<&'a str> {
        if self.get_namespace_separator() == Some("::") {
            return None;
        }
        id.rfind("::").map(|x| &id[x + 2..])
    }

    /// The innermost group the commands are in.
    pub(crate) fn get_current_group(&self) -> EntityId {
        self.stacks
            .iter()
            .rev()
            .find_map(|bag| match bag {
                Bag::Group(group) => Some(*group),
                Bag::Together(_) => None,
            })
            .expect("the root is always on the stack")
    }

    /// The `together` block the commands are directly in.
    pub(crate) fn current_together(&self) -> Option<TogetherId> {
        match self.stacks.last() {
            Some(Bag::Together(together)) => Some(*together),
            _ => None,
        }
    }

    pub(crate) fn get_together(&self, id: TogetherId) -> &Together {
        &self.togethers[id.0]
    }

    /// A name without the quotes, parentheses, brackets or colons around it.
    pub(crate) fn clean_id(id: &str) -> &str {
        if id.chars().count() < 2 {
            return id;
        }
        let unquoted = crate::text::unquoted(id);
        if unquoted.len() != id.len() {
            return unquoted;
        }
        [('(', ')'), ('[', ']'), (':', ':')]
            .into_iter()
            .find_map(|(start, end)| id.strip_prefix(start)?.strip_suffix(end))
            .unwrap_or(id)
    }

    pub(crate) fn set_last_entity(&mut self, last: Option<EntityId>) {
        self.last_entity = last;
    }

    pub(crate) fn get_last_entity(&self) -> Option<EntityId> {
        self.last_entity
    }

    fn new_entity(
        &mut self,
        location: Option<&LineLocation>,
        quark: QuarkId,
        style_builder: Option<Rc<StyleBuilder>>,
        entity_type: EntityType,
    ) -> EntityId {
        let uid = if quark.is_root() {
            "entroot".to_owned()
        } else {
            format!("ent{:04}", self.get_unique_sequence_value())
        };
        let id = EntityId(self.entities.len());
        self.entities.push(Entity::new(
            id,
            quark,
            uid,
            location.cloned(),
            style_builder,
            self.raw_layout,
            entity_type,
        ));
        self.namespace.set_data(quark, id);
        id
    }

    /// A leaf for `ident`, which holds no entity yet, as the latest entity and in the current `together`.
    /// Class-like leaves turn every quark above them that holds nothing into a package.
    pub(crate) fn really_create_leaf(
        &mut self,
        location: Option<&LineLocation>,
        ident: QuarkId,
        display: Display,
        leaf_type: LeafType,
    ) -> EntityId {
        assert!(
            self.quark(ident).get_data().is_none(),
            "{} is already an entity",
            self.quark(ident).get_qualified_name()
        );
        let result = self.create_leaf(location, ident, leaf_type);
        self.last_entity = Some(result);
        let together = self.current_together();
        let entity = self.entity_mut(result);
        entity.together = together;
        entity.display = display;
        if leaf_type.is_like_class() {
            self.eventually_build_phantom_groups(location);
        }
        result
    }

    pub(crate) fn quark_in_context(&mut self, reuse_existing_child: bool, full: &str) -> QuarkId {
        self.quark_in_context_safe(reuse_existing_child, full)
            .unwrap_or_else(|failure| panic!("{}", failure.error))
    }

    /// The quark a name written in the current group means. With a separator, `.a.b` starts from the root,
    /// `a.b` too when `a` is a package below the root, and a plain name already used once elsewhere may
    /// mean that one.
    pub(crate) fn quark_in_context_safe(
        &mut self,
        reuse_existing_child: bool,
        full: &str,
    ) -> Result<QuarkId, Failure> {
        let Some(sep) = self.namespace_separator.clone() else {
            if let Some(result) = self.namespace.first_with_name(full) {
                return Ok(result);
            }
            let current = self.entity(self.get_current_group()).get_quark();
            return Ok(self.namespace.child(current, full));
        };
        if full.ends_with(&sep) || full.contains(&format!("{sep}{sep}")) {
            return Err(Failure {
                error: format!("Bad name since {sep} is a separator"),
                score: 3,
            });
        }
        let current_quark = self.entity(self.get_current_group()).get_quark();
        let root = QuarkId::ROOT;
        if let Some(from_root) = full.strip_prefix(&sep) {
            return Ok(self.namespace.child(root, from_root));
        }
        let Some(x) = full.find(&sep) else {
            if reuse_existing_child
                && self.namespace.count_by_name(full) == 1
                && let Some(by_name) = self.namespace.first_with_name(full)
                && by_name != current_quark
            {
                return Ok(by_name);
            }
            return Ok(self.namespace.child(current_quark, full));
        };
        let first = self.namespace.child_if_exists(root, &full[..x]);
        if let Some(first) = first {
            if self
                .quark(first)
                .get_data()
                .is_some_and(|data| !self.entity(data).is_group())
            {
                return Err(Failure {
                    error: format!("Not a package: {}", &full[..x]),
                    score: 0,
                });
            }
            return Ok(self.namespace.child(root, full));
        }
        Ok(self.namespace.child(current_quark, full))
    }

    /// Every quark below the root that holds nothing yet but has children becomes a package named after it.
    pub(crate) fn eventually_build_phantom_groups(&mut self, location: Option<&LineLocation>) {
        let phantoms: Vec<QuarkId> = self
            .namespace
            .quarks()
            .filter(|quark| {
                let quark = self.quark(*quark);
                quark.get_data().is_none() && quark.count_children() > 0
            })
            .collect();
        for quark in phantoms {
            let display = Display::with_newlines(self.quark(quark).get_name());
            let result = self.create_group(location, quark, GroupType::Package);
            self.entity_mut(result).display = display;
        }
    }

    pub(crate) fn goto_together(&mut self) {
        let together = TogetherId(self.togethers.len());
        self.togethers.push(Together {
            parent: self.current_together(),
        });
        self.stacks.push(Bag::Together(together));
    }

    /// Enters the group `quark` holds, created if needed, as a group of `group_type`.
    pub(crate) fn goto_group(
        &mut self,
        location: Option<&LineLocation>,
        quark: QuarkId,
        display: Display,
        group_type: GroupType,
    ) {
        let group = if let Some(existing) = self.quark(quark).get_data() {
            existing
        } else {
            let result = self.create_group(location, quark, group_type);
            let together = self.current_together();
            let entity = self.entity_mut(result);
            entity.together = together;
            entity.display = display;
            result
        };
        self.entity_mut(group).mute_to_group_type(group_type);
        self.stacks.push(Bag::Group(group));
    }

    /// Leaves the innermost group or `together` block; whether there was one.
    pub(crate) fn end_group(&mut self) -> bool {
        self.stacks.pop().is_some()
    }

    /// The entity of the first quark named `code`.
    pub(crate) fn get_group(&self, code: &str) -> Option<EntityId> {
        self.quark(self.namespace.first_with_name(code)?).get_data()
    }

    pub(crate) fn is_group(&self, code: &str) -> bool {
        self.namespace
            .first_with_name(code)
            .is_some_and(|quark| self.is_group_quark(quark))
    }

    pub(crate) fn is_group_quark(&self, quark: QuarkId) -> bool {
        self.quark(quark)
            .get_data()
            .is_some_and(|data| self.entity(data).is_group())
    }

    /// Whether `entity` shows `portion`: the last `hide` or `show` about it decides. Strict UML shows no
    /// circled characters.
    pub(crate) fn show_portion(&self, portion: EntityPortion, entity: EntityId) -> bool {
        if self.skin().strict_uml_style() && portion == EntityPortion::CircledCharacter {
            return false;
        }
        let entity = self.entity(entity);
        self.hide_or_shows
            .iter()
            .filter(|cmd| cmd.portion == portion && cmd.gender.contains(entity, self))
            .last()
            .is_none_or(|cmd| cmd.show)
    }

    /// The stereotype labels of `entity` that `hide ... stereotype` leaves shown, as written.
    pub(crate) fn get_visible_stereotype_labels(&self, entity: EntityId) -> Option<Vec<String>> {
        let stereotype = self.entity(entity).stereotype.as_ref()?;
        Some(
            stereotype
                .labels_double_comparator()
                .into_iter()
                .filter(|label| self.is_stereotype_label_shown(label))
                .collect(),
        )
    }

    fn is_stereotype_label_shown(&self, stereo_type_label: &str) -> bool {
        self.hide_or_shows
            .iter()
            .filter(|cmd| cmd.portion == EntityPortion::Stereotype)
            .filter(|cmd| {
                cmd.gender
                    .get_gender(self)
                    .is_none_or(|gender| gender == stereo_type_label)
            })
            .last()
            .is_none_or(|cmd| cmd.show)
    }

    pub(crate) fn hide_or_show(
        &mut self,
        gender: &EntityGender,
        portions: EntityPortion,
        show: bool,
    ) {
        for portion in portions.as_set() {
            self.hide_or_shows.push(EntityHideOrShow {
                gender: gender.clone(),
                portion,
                show,
            });
        }
    }

    /// `hide what` or `show what`; inside a group, names are relative to it.
    pub(crate) fn hide_or_show2(&mut self, what: &str, show: bool) {
        let what = self.fix_what(what);
        self.hides2.push(HideOrShow::new(what, show));
    }

    /// `remove what` or `restore what`; inside a group, names are relative to it.
    pub(crate) fn remove_or_restore(&mut self, what: &str, show: bool) {
        let what = self.fix_what(what);
        self.removed.push(HideOrShow::new(what, show));
    }

    fn fix_what(&self, what: &str) -> String {
        if let Some(sep) = self.get_namespace_separator() {
            let current_quark = self.entity(self.get_current_group()).get_quark();
            let qualified_name = self.quark(current_quark).get_qualified_name();
            if !qualified_name.is_empty() {
                return format!("{qualified_name}{sep}{what}");
            }
        }
        what.to_owned()
    }

    /// Whether no link touches `ent`.
    pub(crate) fn is_standalone(&self, ent: EntityId) -> bool {
        !self.get_links().any(|link| link.contains(ent))
    }

    /// The latest link between entities that are not notes.
    pub(crate) fn get_last_link(&self) -> Option<LinkId> {
        self.links_between_non_notes().next()
    }

    /// The two latest links between entities that are not notes, the latest first.
    pub(crate) fn get_two_last_links(&self) -> Option<[LinkId; 2]> {
        let mut links = self.links_between_non_notes();
        Some([links.next()?, links.next()?])
    }

    fn links_between_non_notes(&self) -> impl Iterator<Item = LinkId> + '_ {
        let is_note = |id: EntityId| self.entity(id).get_leaf_type() == Some(LeafType::Note);
        self.link_order.iter().rev().copied().filter(move |link| {
            let link = self.link(*link);
            !is_note(link.get_entity1()) && !is_note(link.get_entity2())
        })
    }

    /// PlantUML's odd margins, kept for compatibility.
    pub(crate) fn get_default_margins() -> ClockwiseTopRightBottomLeft {
        ClockwiseTopRightBottomLeft::top_right_bottom_left(0.0, 5.0, 5.0, 0.0)
    }

    /// The next number of the diagram-wide counter that numbers entities and links.
    pub(crate) fn get_unique_sequence_value(&mut self) -> i32 {
        self.cpt1 += 1;
        self.cpt1
    }

    /// `prefix` and the next number of the diagram-wide counter.
    pub(crate) fn get_unique_sequence(&mut self, prefix: &str) -> String {
        format!("{prefix}{}", self.get_unique_sequence_value())
    }

    /// `prefix` and the next number of the counter each parsing pass restarts.
    pub(crate) fn get_unique_sequence2(&mut self, prefix: &str) -> String {
        self.cpt2 += 1;
        format!("{prefix}{}", self.cpt2)
    }

    pub(crate) fn is_stereotype_removed(&self, stereotype: &Stereotype) -> bool {
        self.removed.iter().fold(false, |result, hide| {
            hide.apply_to_stereotype(result, stereotype)
        })
    }

    /// Whether `hide` commands hide the entity; a note on a single entity shares its fate.
    pub(crate) fn is_hidden(&self, leaf: EntityId) -> bool {
        if self.entity(leaf).is_root() {
            return false;
        }
        if let Some(other) = self.is_note_with_single_link_attached_to(leaf)
            && other != leaf
        {
            return self.is_hidden(other);
        }
        self.hides2
            .iter()
            .fold(false, |hidden, hide| hide.apply(hidden, leaf, self))
    }

    /// Whether `remove` commands remove the entity; a note on a single entity shares its fate.
    pub(crate) fn is_removed(&self, leaf: EntityId) -> bool {
        if self.entity(leaf).is_root() {
            return false;
        }
        if let Some(other) = self.is_note_with_single_link_attached_to(leaf) {
            return self.is_removed(other);
        }
        self.removed
            .iter()
            .fold(false, |result, hide| hide.apply(result, leaf, self))
    }

    /// The entity a note is attached to, when the note has exactly one visible link and it goes to something
    /// other than a note.
    fn is_note_with_single_link_attached_to(&self, note: EntityId) -> Option<EntityId> {
        if self.entity(note).get_leaf_type() != Some(LeafType::Note) {
            return None;
        }
        let mut other = None;
        for link in self.get_links() {
            if link.get_type().is_invisible() || !link.contains(note) {
                continue;
            }
            if other.is_some() {
                return None;
            }
            let found = link.get_other(note);
            if self.entity(found).get_leaf_type() == Some(LeafType::Note) {
                return None;
            }
            other = Some(found);
        }
        other
    }

    /// Like [`Self::is_removed`] by `remove` commands other than `remove @unlinked`.
    pub(crate) fn is_removed_ignore_unlinked(&self, leaf: EntityId) -> bool {
        self.removed
            .iter()
            .filter(|hide| !hide.is_about_unlinked())
            .fold(false, |result, hide| hide.apply(result, leaf, self))
    }

    /// A leaf of `entity_type` for `quark`, numbered next.
    pub(crate) fn create_leaf(
        &mut self,
        location: Option<&LineLocation>,
        quark: QuarkId,
        entity_type: LeafType,
    ) -> EntityId {
        let style_builder = Some(self.skin().current_style_builder());
        self.new_entity(
            location,
            quark,
            style_builder,
            EntityType::Leaf(entity_type),
        )
    }

    /// The entity `quark` holds, or a new group of `group_type` for it.
    pub(crate) fn create_group(
        &mut self,
        location: Option<&LineLocation>,
        quark: QuarkId,
        group_type: GroupType,
    ) -> EntityId {
        if let Some(existing) = self.quark(quark).get_data() {
            return existing;
        }
        let style_builder = Some(self.skin().current_style_builder());
        self.new_entity(
            location,
            quark,
            style_builder,
            EntityType::Group(group_type),
        )
    }

    /// Every leaf, in the order their names were first used.
    pub(crate) fn leafs(&self) -> Vec<EntityId> {
        self.entities_of_quarks(|entity| !entity.is_root() && !entity.is_group())
    }

    /// Every group but the root, in the order their names were first used.
    pub(crate) fn groups(&self) -> Vec<EntityId> {
        self.entities_of_quarks(|entity| !entity.is_root() && entity.is_group())
    }

    /// The root, then every group, in the order their names were first used.
    pub(crate) fn groups_and_root(&self) -> Vec<EntityId> {
        self.entities_of_quarks(Entity::is_group)
    }

    fn entities_of_quarks(&self, keep: impl Fn(&Entity) -> bool) -> Vec<EntityId> {
        self.namespace
            .quarks()
            .filter_map(|quark| self.quark(quark).get_data())
            .filter(|entity| keep(self.entity(*entity)))
            .collect()
    }

    pub(crate) fn inc_raw_layout(&mut self) {
        self.raw_layout += 1;
    }

    /// The diagram's links, in the order they were added.
    pub(crate) fn get_links(&self) -> impl Iterator<Item = &Link> + '_ {
        self.link_order.iter().map(|id| self.link(*id))
    }

    /// The ids of the diagram's links, in the order they were added.
    pub(crate) fn get_link_ids(&self) -> &[LinkId] {
        &self.link_order
    }

    /// A link numbered next, which the diagram only shows once added.
    pub(crate) fn new_link(
        &mut self,
        location: Option<&LineLocation>,
        cl1: EntityId,
        cl2: EntityId,
        link_type: LinkType,
        link_arg: LinkArg,
    ) -> LinkId {
        let uid = self.get_unique_sequence("lnk");
        let id = LinkId(self.links.len());
        let style_builder = self.skin().current_style_builder();
        self.links.push(Link::new(
            id,
            uid,
            location.cloned(),
            style_builder,
            cl1,
            cl2,
            link_type,
            link_arg,
        ));
        id
    }

    /// The same link drawn from its other end, numbered next; `-left->` and `-up->` arrows are made so.
    pub(crate) fn get_inv(&mut self, link: LinkId) -> LinkId {
        let uid = self.get_unique_sequence("lnk");
        let id = LinkId(self.links.len());
        let inv = self.link(link).get_inv(id, uid);
        self.links.push(inv);
        id
    }

    /// Ports of the link's entities, which they learn the names of.
    pub(crate) fn set_port_members(
        &mut self,
        link: LinkId,
        port1: Option<String>,
        port2: Option<String>,
    ) {
        let (cl1, cl2) = (self.link(link).get_entity1(), self.link(link).get_entity2());
        if let Some(port1) = &port1 {
            self.entity_mut(cl1).add_port_short_name(port1.clone());
        }
        if let Some(port2) = &port2 {
            self.entity_mut(cl2).add_port_short_name(port2.clone());
        }
        self.link_mut(link).set_ports(port1, port2);
    }

    /// Adds the link, unless it is `single` and the same two entities are linked already.
    pub(crate) fn add_link(&mut self, link: LinkId) {
        if self.link(link).is_single() && self.contains_similar_link(link) {
            return;
        }
        self.link_order.push(link);
    }

    fn contains_similar_link(&self, other: LinkId) -> bool {
        let other = self.link(other);
        self.get_links().any(|link| other.same_connections(link))
    }

    /// # Panics
    ///
    /// If the diagram does not show the link.
    pub(crate) fn remove_link(&mut self, link: LinkId) {
        let index = self
            .link_order
            .iter()
            .position(|existing| *existing == link)
            .expect("only links of the diagram are removed");
        self.link_order.remove(index);
    }

    /// Every quark, the root first, in creation order.
    pub(crate) fn quarks(&self) -> impl Iterator<Item = QuarkId> + use<> {
        self.namespace.quarks()
    }

    pub(crate) fn get_root_group(&self) -> EntityId {
        self.entities[0].id()
    }

    pub(crate) fn first_with_name(&self, full: &str) -> Option<QuarkId> {
        self.namespace.first_with_name(full)
    }

    pub(crate) fn count_by_name(&self, full: &str) -> usize {
        self.namespace.count_by_name(full)
    }

    /// A group laid out on its own becomes a leaf of `leaf_type` drawn by its layout, which takes the links
    /// inside it along (the model part of PlantUML's `Entity.overrideImage`).
    pub(crate) fn override_image(&mut self, group: EntityId, leaf_type: LeafType) {
        let inner: Vec<LinkId> = self
            .link_order
            .iter()
            .copied()
            .filter(|link| is_pure_inner_link12(self.entity(group), self.link(*link), self))
            .collect();
        for link in inner {
            self.remove_link(link);
        }
        let entity = self.entity_mut(group);
        assert!(entity.is_group(), "only groups are laid out on their own");
        entity.url = None;
        entity.mute_to_type(leaf_type);
    }
}
