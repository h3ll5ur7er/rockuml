//! The images of entities (PlantUML's `svek.image` package).

mod entity_image_note;
mod entity_image_note_link;
mod entity_image_tips;
mod opale;

pub(crate) use entity_image_note::{EntityImageNote, OpaleLink};
pub(crate) use entity_image_note_link::EntityImageNoteLink;
pub(crate) use entity_image_tips::EntityImageTips;
pub(crate) use opale::{get_corner, get_polygon_normal};
