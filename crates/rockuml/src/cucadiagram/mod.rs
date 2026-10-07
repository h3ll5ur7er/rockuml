//! The bodies of entities: members of classes and objects, entries of maps, JSON data, and the text blocks
//! that draw them (PlantUML's `cucadiagram` package).

mod bodier;
mod member;

pub(crate) use bodier::{Bodier, get_linked_entry};
pub(crate) use member::Member;
