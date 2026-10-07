//! An element of an entity diagram: a leaf like a class or a state, or a group like a package (PlantUML's
//! `Entity`). Entities live in their diagram's arena; what depends on the rest of the diagram takes it as an
//! argument.

use std::rc::Rc;

use super::{
    CucaNote, DisplayPositioned, EntityPosition, GroupType, LeafType, Position, Tip, TogetherId,
    is_pure_inner_link3,
};
use crate::color::Colors;
use crate::creole::Display;
use crate::decoration::symbol::{PackageStyle, USymbol, USymbols};
use crate::diagram::cuca::CucaDiagram;
use crate::java::{JavaHashSet, string_hash_code};
use crate::klimt::VerticalAlignment;
use crate::klimt::url::Url;
use crate::plasma::QuarkId;
use crate::stereo::{Stereotag, Stereotype};
use crate::style::StyleBuilder;
use crate::text::LineLocation;

/// An entity, by its place in its diagram's arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct EntityId(pub(crate) usize);

/// What an entity is: exactly one of PlantUML's `leafType` and `groupType` is set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntityType {
    Leaf(LeafType),
    Group(GroupType),
}

#[derive(Clone)]
pub(crate) struct Entity {
    id: EntityId,
    quark: QuarkId,
    uid: String,
    location: Option<LineLocation>,
    /// The style rules in force when the entity was declared; the root has none.
    style_builder: Option<Rc<StyleBuilder>>,
    raw_layout: i32,
    leaf_or_group: EntityType,
    pub display: Display,
    pub stereotype: Option<Stereotype>,
    pub url: Option<Url>,
    pub generic: Option<String>,
    /// A legend drawn inside a group.
    pub legend: Option<(DisplayPositioned, VerticalAlignment)>,
    tags: Vec<Stereotag>,
    notes_top: Vec<CucaNote>,
    notes_bottom: Vec<CucaNote>,
    pub together: Option<TogetherId>,
    packed: bool,
    pub is_static: bool,
    pub colors: Colors,
    /// The symbol a description element or a group is drawn as; see [`Entity::get_usymbol`].
    pub usymbol: Option<USymbol>,
    /// By member, in the order members were first given a tip.
    tips: Vec<(String, Tip)>,
    port_short_names: JavaHashSet<String>,
    /// The character a state's concurrent regions were separated with, `--` or `||`.
    pub concurrent_separator: Option<char>,
}

impl Entity {
    /// Only diagrams create entities, numbering them as they go.
    pub(crate) fn new(
        id: EntityId,
        quark: QuarkId,
        uid: String,
        location: Option<LineLocation>,
        style_builder: Option<Rc<StyleBuilder>>,
        raw_layout: i32,
        entity_type: EntityType,
    ) -> Self {
        Self {
            id,
            quark,
            uid,
            location,
            style_builder,
            raw_layout,
            leaf_or_group: entity_type,
            display: Display::default(),
            stereotype: None,
            url: None,
            generic: None,
            legend: None,
            tags: Vec::new(),
            notes_top: Vec::new(),
            notes_bottom: Vec::new(),
            together: None,
            packed: false,
            is_static: false,
            colors: Colors::default(),
            usymbol: None,
            tips: Vec::new(),
            port_short_names: JavaHashSet::default(),
            concurrent_separator: None,
        }
    }

    pub(crate) fn id(&self) -> EntityId {
        self.id
    }

    pub(crate) fn get_quark(&self) -> QuarkId {
        self.quark
    }

    /// `ent0001`, `ent0002`... in creation order, `entroot` for the root.
    pub(crate) fn get_uid(&self) -> &str {
        &self.uid
    }

    pub(crate) fn get_location(&self) -> Option<&LineLocation> {
        self.location.as_ref()
    }

    pub(crate) fn style_builder(&self) -> Option<&Rc<StyleBuilder>> {
        self.style_builder.as_ref()
    }

    pub(crate) fn get_raw_layout(&self) -> i32 {
        self.raw_layout
    }

    /// `None` for groups.
    pub(crate) fn get_leaf_type(&self) -> Option<LeafType> {
        match self.leaf_or_group {
            EntityType::Leaf(leaf_type) => Some(leaf_type),
            EntityType::Group(_) => None,
        }
    }

    /// # Panics
    ///
    /// For leaves.
    pub(crate) fn get_group_type(&self) -> GroupType {
        match self.leaf_or_group {
            EntityType::Group(group_type) => group_type,
            EntityType::Leaf(_) => panic!("{} is no group", self.uid),
        }
    }

    pub(crate) fn is_group(&self) -> bool {
        matches!(self.leaf_or_group, EntityType::Group(_))
    }

    pub(crate) fn mute_to_type(&mut self, new_type: LeafType) {
        self.leaf_or_group = EntityType::Leaf(new_type);
    }

    /// Changes the type when both are class-like (or the old one is still unknown); an object can replace a
    /// class. Whether the type is now `new_type` (PlantUML's `muteToType(LeafType, USymbol)`).
    pub(crate) fn mute_to_type_if_compatible(&mut self, new_type: LeafType) -> bool {
        use LeafType::{
            AbstractClass, Annotation, Class, Dataclass, Enum, Interface, Object, Record,
        };
        let leaf_type = self.get_leaf_type();
        if leaf_type != Some(LeafType::StillUnknown) {
            if leaf_type == Some(new_type) {
                return true;
            }
            let class_like = |leaf_type: LeafType| {
                matches!(
                    leaf_type,
                    Annotation | AbstractClass | Class | Enum | Interface | Record | Dataclass
                )
            };
            if !leaf_type.is_some_and(class_like) || !(class_like(new_type) || new_type == Object) {
                return false;
            }
        }
        self.leaf_or_group = EntityType::Leaf(new_type);
        true
    }

    pub(crate) fn mute_to_group_type(&mut self, new_type: GroupType) {
        self.leaf_or_group = EntityType::Group(new_type);
    }

    /// Use cases and circles are always drawn as their symbol.
    pub(crate) fn get_usymbol(&self) -> Option<USymbol> {
        match self.get_leaf_type() {
            Some(LeafType::Usecase) => Some(USymbols::USECASE),
            Some(LeafType::UsecaseBusiness) => Some(USymbols::USECASE_BUSINESS),
            Some(LeafType::Circle) => Some(USymbols::INTERFACE),
            _ => self.usymbol,
        }
    }

    /// The style a stereotype like `<<Node>>` gives a package.
    pub(crate) fn get_package_style(&self) -> Option<PackageStyle> {
        PackageStyle::from_stereotype(&self.stereotype.as_ref()?.label_double_comparator())
    }

    pub(crate) fn add_note(&mut self, note: Display, position: Position, colors: Colors) {
        match position {
            Position::Top => self.notes_top.push(CucaNote::build(note, position, colors)),
            Position::Bottom => self
                .notes_bottom
                .push(CucaNote::build(note, position, colors)),
            Position::Left | Position::Right => {}
        }
    }

    /// # Panics
    ///
    /// For sides other than top and bottom, which carry no notes.
    pub(crate) fn get_notes(&self, position: Position) -> &[CucaNote] {
        match position {
            Position::Top => &self.notes_top,
            Position::Bottom => &self.notes_bottom,
            Position::Left | Position::Right => {
                panic!("entities keep notes on top and bottom only")
            }
        }
    }

    pub(crate) fn add_stereotag(&mut self, tag: Stereotag) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    pub(crate) fn stereotags(&self) -> &[Stereotag] {
        &self.tags
    }

    pub(crate) fn get_entity_position(&self) -> EntityPosition {
        match self.get_leaf_type() {
            Some(LeafType::Portin) => EntityPosition::Portin,
            Some(LeafType::Portout) => EntityPosition::Portout,
            Some(LeafType::State) if !self.is_root() => self
                .stereotype
                .as_ref()
                .map_or(EntityPosition::Normal, |stereotype| {
                    EntityPosition::from_stereotype(&stereotype.label_double_comparator())
                }),
            _ => EntityPosition::Normal,
        }
    }

    pub(crate) fn is_root(&self) -> bool {
        self.quark.is_root()
    }

    pub(crate) fn is_hidden(&self, diagram: &CucaDiagram) -> bool {
        if self.is_root() {
            return false;
        }
        match self.get_parent_container(diagram) {
            Some(parent) if parent == self.id => return false,
            Some(parent) if diagram.entity(parent).is_hidden(diagram) => return true,
            _ => {}
        }
        diagram.is_hidden(self.id)
    }

    pub(crate) fn is_removed(&self, diagram: &CucaDiagram) -> bool {
        if self.is_root() {
            return false;
        }
        match self.get_parent_container(diagram) {
            Some(parent) if parent == self.id => return false,
            Some(parent) if diagram.entity(parent).is_removed(diagram) => return true,
            _ => {}
        }
        diagram.is_removed(self.id)
    }

    /// Whether neither the entity nor anything inside it has a visible link to an entity not removed.
    pub(crate) fn is_alone_and_unlinked(&self, diagram: &CucaDiagram) -> bool {
        if self.is_group() {
            return diagram
                .quark(self.quark)
                .get_children()
                .iter()
                .filter_map(|child| diagram.quark(*child).get_data())
                .all(|child| diagram.entity(child).is_alone_and_unlinked(diagram));
        }
        !diagram.get_links().any(|link| {
            link.contains(self.id)
                && !diagram.is_removed_ignore_unlinked(link.get_other(self.id))
                && !link.get_type().is_invisible()
        })
    }

    pub(crate) fn put_tip(
        &mut self,
        member: String,
        display: Display,
        colors: Colors,
        stereotype: Option<Stereotype>,
    ) {
        let tip = Tip {
            display,
            colors,
            stereotype,
        };
        match self
            .tips
            .iter_mut()
            .find(|(existing, _)| *existing == member)
        {
            Some(entry) => entry.1 = tip,
            None => self.tips.push((member, tip)),
        }
    }

    pub(crate) fn get_tips(&self) -> &[(String, Tip)] {
        &self.tips
    }

    /// In Java's `HashSet` order.
    pub(crate) fn get_port_short_names(&self) -> impl Iterator<Item = &String> {
        self.port_short_names.iter()
    }

    pub(crate) fn add_port_short_name(&mut self, port_short_name: String) {
        let hash = string_hash_code(&port_short_name);
        self.port_short_names.insert(port_short_name, hash);
    }

    /// The group the entity is in; `None` for the root, or below a quark no entity holds yet.
    pub(crate) fn get_parent_container(&self, diagram: &CucaDiagram) -> Option<EntityId> {
        let parent = diagram.quark(self.quark).get_parent()?;
        diagram.quark(parent).get_data()
    }

    /// The leaves right inside the group, in creation order.
    pub(crate) fn leafs(&self, diagram: &CucaDiagram) -> Vec<EntityId> {
        self.children(diagram, false)
    }

    /// The groups right inside the group, in creation order.
    pub(crate) fn groups(&self, diagram: &CucaDiagram) -> Vec<EntityId> {
        self.children(diagram, true)
    }

    fn children(&self, diagram: &CucaDiagram, groups: bool) -> Vec<EntityId> {
        diagram
            .quark(self.quark)
            .get_children()
            .iter()
            .filter_map(|child| diagram.quark(*child).get_data())
            .filter(|child| diagram.entity(*child).is_group() == groups)
            .collect()
    }

    /// Counts the names below the entity, whether entities hold them yet or not.
    pub(crate) fn count_children(&self, diagram: &CucaDiagram) -> usize {
        diagram.quark(self.quark).count_children()
    }

    /// Whether everything inside the group is removed.
    pub(crate) fn is_empty(&self, diagram: &CucaDiagram) -> bool {
        diagram
            .quark(self.quark)
            .get_children()
            .iter()
            .all(|child| {
                diagram
                    .quark(*child)
                    .get_data()
                    .is_some_and(|child| diagram.is_removed(child))
            })
    }

    pub(crate) fn get_name<'a>(&self, diagram: &'a CucaDiagram) -> &'a str {
        diagram.quark(self.quark).get_name()
    }

    /// Whether the group can be laid out on its own: no link crosses its border and nothing sits on it.
    pub(crate) fn is_autarkic(&self, diagram: &CucaDiagram) -> bool {
        match self.get_group_type() {
            GroupType::Package => return false,
            GroupType::InnerActivity
            | GroupType::ConcurrentActivity
            | GroupType::ConcurrentState => {
                return true;
            }
            _ => {}
        }
        diagram
            .get_links()
            .all(|link| is_pure_inner_link3(self, link, diagram))
            && self
                .leafs(diagram)
                .iter()
                .all(|leaf| diagram.entity(*leaf).get_entity_position() == EntityPosition::Normal)
    }

    /// Whether the group only holds one group with something in it, and nothing links to it, so that the two
    /// can show as one, named `outer.inner`.
    pub(crate) fn can_be_packed(&self, diagram: &CucaDiagram) -> bool {
        if self.packed || self.count_children(diagram) != 1 || !self.leafs(diagram).is_empty() {
            return false;
        }
        if diagram.get_links().any(|link| link.contains(self.id)) {
            return false;
        }
        let child = self.groups(diagram)[0];
        diagram.entity(child).count_children(diagram) != 0
    }

    pub(crate) fn set_packed(&mut self) {
        self.packed = true;
    }

    pub(crate) fn is_packed(&self) -> bool {
        self.packed
    }
}
