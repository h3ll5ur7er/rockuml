//! What class, object and description diagrams add to `CucaDiagram`: packing nested packages, and the
//! association classes of class and object diagrams (PlantUML's `AbstractEntityDiagram` and
//! `AbstractClassOrObjectDiagram`).

use super::CucaDiagram;
use crate::abel::{DisplayPositioned, EntityId, LeafType, LinkArg, LinkId, NoteLinkStrategy};
use crate::command::{CommandError, CommandResult};
use crate::creole::Display;
use crate::decoration::{LinkDecor, LinkType};
use crate::diagram::titled::Titled;
use crate::klimt::VerticalAlignment;
use crate::text::{LineLocation, without_quotes_or_brackets};

impl CucaDiagram {
    /// A package holding nothing but one package with something in it shows as one, named `outer.inner`.
    pub(crate) fn pack_some_package(&mut self) {
        let separator = self.get_namespace_separator().unwrap_or(".").to_owned();
        loop {
            let mut changed = false;
            for group in self.groups() {
                let Some(child) = self.entity(group).packable_child(self) else {
                    continue;
                };
                let outer = self.entity(group).display.lines().first();
                let appended = format!("{}{separator}", outer.map_or("", String::as_str));
                let child = self.entity_mut(child);
                child.display = child.display.append_first_line(&appended);
                self.entity_mut(group).set_packed();
                changed = true;
            }
            if !changed {
                return;
            }
        }
    }
}

/// The diagram of class and object diagrams, which names nest with `.` and which have association classes.
pub(crate) struct AbstractClassOrObjectDiagram {
    pub(in crate::diagram) cuca: CucaDiagram,
    associations: Vec<Association>,
}

/// An association class: a point on the link between two entities, linked to a third.
struct Association {
    entity1: EntityId,
    entity2: EntityId,
    associed: EntityId,
    point: EntityId,
    existing_link: Option<LinkId>,
    entity1_to_point: Option<LinkId>,
    point_to_entity2: Option<LinkId>,
    point_to_associed: Option<LinkId>,
    /// The association sharing this one's point, by its place in the diagram's associations.
    other: Option<usize>,
}

impl AbstractClassOrObjectDiagram {
    pub(in crate::diagram) fn new(titled: Titled) -> Self {
        let mut cuca = CucaDiagram::new(titled);
        cuca.set_namespace_separator(Some("."));
        Self {
            cuca,
            associations: Vec::new(),
        }
    }

    /// Cuts the latest link between `entity1` and `entity2` in two at `node`; whether there was one.
    pub(crate) fn insert_between(
        &mut self,
        location: Option<&LineLocation>,
        entity1: EntityId,
        entity2: EntityId,
        node: EntityId,
    ) -> bool {
        let Some(link) = self.found_link(entity1, entity2) else {
            return false;
        };
        let existing = self.cuca.link(link);
        let link_type = existing.get_type();
        let label = existing.get_label().cloned();
        let length = existing.get_length();
        let quantifiers = (
            existing.get_quantifier1().map(str::to_owned),
            existing.get_quantifier2().map(str::to_owned),
        );
        let distance_angle = distance_angle(existing.get_link_arg());
        let l1 = self.cuca.new_link(
            location,
            entity1,
            node,
            link_type,
            LinkArg::build(label.clone(), length)
                .with_quantifier(quantifiers.0, None)
                .with_distance_angle(distance_angle.0.clone(), distance_angle.1.clone()),
        );
        let l2 = self.cuca.new_link(
            location,
            node,
            entity2,
            link_type,
            LinkArg::build(label, length)
                .with_quantifier(None, quantifiers.1)
                .with_distance_angle(distance_angle.0, distance_angle.1),
        );
        self.cuca.add_link(l1);
        self.cuca.add_link(l2);
        self.cuca.remove_link(link);
        true
    }

    /// The latest link between the two entities, either way round.
    fn found_link(&self, entity1: EntityId, entity2: EntityId) -> Option<LinkId> {
        self.cuca
            .get_link_ids()
            .iter()
            .rev()
            .copied()
            .find(|link| self.cuca.link(*link).is_between(entity1, entity2))
    }

    /// How many lollipops sit right next to `entity`.
    pub(crate) fn get_nb_of_hozizontal_lollipop(&self, entity: EntityId) -> usize {
        self.cuca
            .get_links()
            .filter(|link| {
                link.get_length() == 1
                    && link.contains(entity)
                    && (link.contains_type(LeafType::LollipopFull, &self.cuca)
                        || link.contains_type(LeafType::LollipopHalf, &self.cuca))
            })
            .count()
    }

    /// `(A, B) .. (C, D)`: links points on the links between both pairs.
    pub(crate) fn association_class_between_links(
        &mut self,
        location: Option<&LineLocation>,
        (entity1_a, entity1_b): (EntityId, EntityId),
        (entity2_a, entity2_b): (EntityId, EntityId),
        link_type: LinkType,
        label: Option<Display>,
    ) -> CommandResult {
        let same1 = self.get_existing_associated_points(entity1_a, entity1_b);
        let same2 = self.get_existing_associated_points(entity2_a, entity2_b);
        if !same1.is_empty() || !same2.is_empty() {
            return Err(CommandError::new("Cannot link two associations points"));
        }
        let tmp1 = self.cuca.get_unique_sequence("apoint");
        let tmp2 = self.cuca.get_unique_sequence("apoint");
        let current = self.cuca.entity(self.cuca.get_current_group()).get_quark();
        let code1 = self.cuca.child(current, &tmp1);
        let point1 = self.cuca.really_create_leaf(
            location,
            code1,
            Display::with_newlines(""),
            LeafType::PointForAssociation,
        );
        let code2 = self.cuca.child(current, &tmp2);
        let point2 = self.cuca.really_create_leaf(
            location,
            code2,
            Display::with_newlines(""),
            LeafType::PointForAssociation,
        );
        self.insert_point_between(location, entity1_a, entity1_b, point1);
        self.insert_point_between(location, entity2_a, entity2_b, point2);
        let point1_to_point2 = self.cuca.new_link(
            location,
            point1,
            point2,
            link_type,
            LinkArg::build(label, 1),
        );
        self.cuca.add_link(point1_to_point2);
        Ok(())
    }

    fn insert_point_between(
        &mut self,
        location: Option<&LineLocation>,
        entity1_a: EntityId,
        entity1_b: EntityId,
        point1: EntityId,
    ) {
        let existing_link1 = self.take_existing_link(location, entity1_a, entity1_b);
        let existing = self.cuca.link(existing_link1);
        let (entity1real, entity2real) = if existing.is_inverted() {
            (existing.get_entity2(), existing.get_entity1())
        } else {
            (existing.get_entity1(), existing.get_entity2())
        };
        let link_type = existing.get_type();
        let length = existing.get_length();
        let link_arrow = existing.get_link_arrow();
        let first = LinkArg::build(existing.get_label().cloned(), length)
            .with_quantifier(existing.get_quantifier1().map(str::to_owned), None);
        let second = LinkArg::no_display(length)
            .with_quantifier(None, existing.get_quantifier2().map(str::to_owned));
        let (distance, angle) = distance_angle(existing.get_link_arg());
        let entity1_to_point = self.cuca.new_link(
            location,
            entity1real,
            point1,
            link_type.get_part2(),
            first.with_distance_angle(distance.clone(), angle.clone()),
        );
        self.cuca.link_mut(entity1_to_point).link_arrow = link_arrow;
        let point_to_entity2 = self.cuca.new_link(
            location,
            point1,
            entity2real,
            link_type.get_part1(),
            second.with_distance_angle(distance, angle),
        );
        self.cuca.add_link(entity1_to_point);
        self.cuca.add_link(point_to_entity2);
    }

    /// The latest link between the two entities, taken out of the diagram, or a new plain one two ranks long
    /// that never joins it.
    fn take_existing_link(
        &mut self,
        location: Option<&LineLocation>,
        entity1: EntityId,
        entity2: EntityId,
    ) -> LinkId {
        match self.found_link(entity1, entity2) {
            Some(existing) => {
                self.cuca.remove_link(existing);
                existing
            }
            None => self.cuca.new_link(
                location,
                entity1,
                entity2,
                LinkType::new(LinkDecor::None, LinkDecor::None),
                LinkArg::no_display(2),
            ),
        }
    }

    /// `(A, B) .. C`: links a point on the link between `entity1` and `entity2` to `associed`, from the point
    /// in `mode` 1 and to it otherwise. A second association class on the same pair gets a point of its own;
    /// there is no third.
    #[expect(clippy::too_many_arguments, reason = "PlantUML's method")]
    pub(crate) fn association_class(
        &mut self,
        location: Option<&LineLocation>,
        mode: i32,
        entity1: EntityId,
        entity2: EntityId,
        associed: EntityId,
        link_type: LinkType,
        label: Option<Display>,
    ) -> CommandResult {
        let same = self.get_existing_associated_points(entity1, entity2);
        match same.as_slice() {
            [] => {
                let mut association = self.new_association(location, entity1, entity2, associed)?;
                self.create_new(&mut association, location, mode, link_type, label);
                self.associations.push(association);
            }
            [first] => {
                let association = self.create_second_association(location, *first, associed)?;
                let index = self.associations.len();
                self.associations.push(association);
                self.create_in_second(index, location, link_type, label);
            }
            _ => return Err(CommandError::new("Cannot have more than 2 assocications")),
        }
        Ok(())
    }

    fn get_existing_associated_points(&self, entity1: EntityId, entity2: EntityId) -> Vec<usize> {
        (0..self.associations.len())
            .filter(|index| self.associations[*index].same_couple(entity1, entity2))
            .collect()
    }

    fn new_association(
        &mut self,
        location: Option<&LineLocation>,
        entity1: EntityId,
        entity2: EntityId,
        associed: EntityId,
    ) -> Result<Association, CommandError> {
        let id_short = self.cuca.get_unique_sequence("apoint");
        let parent1 = self
            .cuca
            .quark(self.cuca.entity(entity1).get_quark())
            .get_parent();
        let parent2 = self
            .cuca
            .quark(self.cuca.entity(entity2).get_quark())
            .get_parent();
        let quark = match parent1 {
            Some(parent) if parent1 == parent2 => self.cuca.child(parent, &id_short),
            _ => self
                .cuca
                .quark_in_context(true, without_quotes_or_brackets(&id_short))?,
        };
        let point = self.cuca.really_create_leaf(
            location,
            quark,
            Display::with_newlines(""),
            LeafType::PointForAssociation,
        );
        Ok(Association {
            entity1,
            entity2,
            associed,
            point,
            existing_link: None,
            entity1_to_point: None,
            point_to_entity2: None,
            point_to_associed: None,
            other: None,
        })
    }

    fn create_second_association(
        &mut self,
        location: Option<&LineLocation>,
        first: usize,
        associed2: EntityId,
    ) -> Result<Association, CommandError> {
        let (entity1, entity2) = (
            self.associations[first].entity1,
            self.associations[first].entity2,
        );
        let mut result = self.new_association(location, entity1, entity2, associed2)?;
        let existing = &self.associations[first];
        result.existing_link = existing.existing_link;
        result.other = Some(first);
        let existing_length = existing
            .existing_link
            .map(|link| self.cuca.link(link).get_length());
        if existing_length == Some(1) {
            let links = [
                (existing.entity1_to_point, 2),
                (existing.point_to_entity2, 2),
                (existing.point_to_associed, 1),
            ];
            for (link, length) in links {
                if let Some(link) = link {
                    self.cuca.link_mut(link).set_length(length);
                }
            }
        }
        Ok(result)
    }

    fn create_new(
        &mut self,
        association: &mut Association,
        location: Option<&LineLocation>,
        mode: i32,
        link_type: LinkType,
        label: Option<Display>,
    ) {
        let (entity1, entity2) = (association.entity1, association.entity2);
        let existing_link = self.take_existing_link(location, entity1, entity2);
        association.existing_link = Some(existing_link);
        let existing = self.cuca.link(existing_link);
        let (entity1real, entity2real) = (existing.get_entity1(), existing.get_entity2());
        let existing_type = existing.get_type();
        let existing_length = existing.get_length();
        let link_arrow = existing.get_link_arrow();
        let first = LinkArg::build(existing.get_label().cloned(), existing_length)
            .with_quantifier(existing.get_quantifier1().map(str::to_owned), None);
        let second = LinkArg::no_display(existing_length)
            .with_quantifier(None, existing.get_quantifier2().map(str::to_owned));
        let (distance, angle) = distance_angle(existing.get_link_arg());
        let point = association.point;
        let entity1_to_point = self.cuca.new_link(
            location,
            entity1real,
            point,
            existing_type.get_part2(),
            first.with_distance_angle(distance.clone(), angle.clone()),
        );
        self.cuca.link_mut(entity1_to_point).link_arrow = link_arrow;
        let point_to_entity2 = self.cuca.new_link(
            location,
            point,
            entity2real,
            existing_type.get_part1(),
            second.with_distance_angle(distance, angle),
        );
        let length = if (existing_length == 1 && entity1 != entity2)
            || (existing_length == 2 && entity1 == entity2)
        {
            2
        } else {
            1
        };
        if length == 1 {
            self.add_note_from(entity1_to_point, existing_link, NoteLinkStrategy::Normal);
        } else {
            self.add_note_from(
                entity1_to_point,
                existing_link,
                NoteLinkStrategy::HalfPrintedFull,
            );
            self.add_note_from(
                point_to_entity2,
                existing_link,
                NoteLinkStrategy::HalfNotPrinted,
            );
        }
        self.cuca.add_link(entity1_to_point);
        self.cuca.add_link(point_to_entity2);
        let (from, to) = if mode == 1 {
            (point, association.associed)
        } else {
            (association.associed, point)
        };
        let point_to_associed =
            self.cuca
                .new_link(location, from, to, link_type, LinkArg::build(label, length));
        self.cuca.add_link(point_to_associed);
        association.entity1_to_point = Some(entity1_to_point);
        association.point_to_entity2 = Some(point_to_entity2);
        association.point_to_associed = Some(point_to_associed);
    }

    fn add_note_from(&mut self, link: LinkId, from: LinkId, strategy: NoteLinkStrategy) {
        if let Some(note) = &self.cuca.link(from).note {
            let note = note.with_strategy(strategy);
            self.cuca.link_mut(link).note = Some(note);
        }
    }

    fn create_in_second(
        &mut self,
        index: usize,
        location: Option<&LineLocation>,
        link_type: LinkType,
        label: Option<Display>,
    ) {
        let association = &self.associations[index];
        let (entity1, entity2, point, associed) = (
            association.entity1,
            association.entity2,
            association.point,
            association.associed,
        );
        let other = association.other.expect("a second association has a first");
        let existing_link = self.take_existing_link(location, entity1, entity2);
        let existing = self.cuca.link(existing_link);
        let existing_type = existing.get_type();
        let first = LinkArg::build(existing.get_label().cloned(), 2)
            .with_quantifier(existing.get_quantifier1().map(str::to_owned), None);
        let second = LinkArg::no_display(2)
            .with_quantifier(None, existing.get_quantifier2().map(str::to_owned));
        let (distance, angle) = distance_angle(existing.get_link_arg());
        let entity1_to_point = self.cuca.new_link(
            location,
            entity1,
            point,
            existing_type.get_part2(),
            first.with_distance_angle(distance.clone(), angle.clone()),
        );
        let point_to_entity2 = self.cuca.new_link(
            location,
            point,
            entity2,
            existing_type.get_part1(),
            second.with_distance_angle(distance, angle),
        );
        self.cuca.add_link(entity1_to_point);
        self.cuca.add_link(point_to_entity2);
        let other_point_to_associed = self.associations[other]
            .point_to_associed
            .expect("the first association is complete");
        let starts_at_point = self
            .cuca
            .entity(self.cuca.link(other_point_to_associed).get_entity1())
            .get_leaf_type()
            == Some(LeafType::PointForAssociation);
        if starts_at_point {
            self.cuca.remove_link(other_point_to_associed);
            let inv = self.cuca.get_inv(other_point_to_associed);
            self.associations[other].point_to_associed = Some(inv);
            self.cuca.add_link(inv);
        }
        let point_to_associed = self.cuca.new_link(
            location,
            point,
            associed,
            link_type,
            LinkArg::build(label, 1),
        );
        self.cuca.add_link(point_to_associed);
        let other_point = self.associations[other].point;
        let lnode = self.cuca.new_link(
            location,
            other_point,
            point,
            LinkType::new(LinkDecor::None, LinkDecor::None),
            LinkArg::no_display(1),
        );
        self.cuca.link_mut(lnode).set_invis(true);
        self.cuca.add_link(lnode);
        let association = &mut self.associations[index];
        association.existing_link = Some(existing_link);
        association.entity1_to_point = Some(entity1_to_point);
        association.point_to_entity2 = Some(point_to_entity2);
        association.point_to_associed = Some(point_to_associed);
    }

    /// A legend inside a package goes with the package.
    pub(crate) fn set_legend(&mut self, legend: DisplayPositioned, vertical: VerticalAlignment) {
        let current_group = self.cuca.get_current_group();
        if self.cuca.entity(current_group).is_root() {
            self.cuca.titled.set_legend(legend, vertical);
            return;
        }
        self.cuca.entity_mut(current_group).legend = Some((legend, vertical));
    }
}

impl Association {
    fn same_couple(&self, entity1: EntityId, entity2: EntityId) -> bool {
        (self.entity1 == entity1 && self.entity2 == entity2)
            || (self.entity1 == entity2 && self.entity2 == entity1)
    }
}

fn distance_angle(link_arg: &LinkArg) -> (Option<String>, Option<String>) {
    (
        link_arg.get_labeldistance().map(str::to_owned),
        link_arg.get_labelangle().map(str::to_owned),
    )
}
