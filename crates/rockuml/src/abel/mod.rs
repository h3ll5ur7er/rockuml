//! The elements of entity diagrams: entities, which are leaves or groups, the links between them, and what
//! hangs on them (PlantUML's `abel` package).

mod cuca_note;
mod entity;
mod entity_gender;
mod entity_position;
mod entity_utils;
mod leaf_type;
mod link;

pub(crate) use cuca_note::{CucaNote, NoteLinkStrategy, Position, Tip};
pub(crate) use entity::{Entity, EntityId, EntityType};
pub(crate) use entity_gender::EntityGender;
pub(crate) use entity_position::{EntityPortion, EntityPosition};
pub(crate) use entity_utils::{is_pure_inner_link3, is_pure_inner_link12};
pub(crate) use leaf_type::{GroupType, LeafType};
pub(crate) use link::{Link, LinkArg, LinkArrow, LinkId};

use crate::creole::Display;
use crate::klimt::HorizontalAlignment;
use crate::text::LineLocation;

/// A text placed around a diagram or a group, like a title or a legend, and the source line that wrote it.
#[derive(Clone)]
pub(crate) struct DisplayPositioned {
    pub display: Display,
    pub alignment: HorizontalAlignment,
    pub location: Option<LineLocation>,
}

/// A `together { ... }` block, by its place in the diagram's list of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TogetherId(pub(crate) usize);

/// What a diagram's commands are currently inside: a group, or a `together` block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Bag {
    Group(EntityId),
    Together(TogetherId),
}
