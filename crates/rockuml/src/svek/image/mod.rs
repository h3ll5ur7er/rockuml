//! The images of entities (PlantUML's `svek.image` package).

mod chen;
mod entity_image_note;
mod entity_image_note_link;
mod entity_image_tips;
mod opale;

pub(crate) use chen::{
    EntityImageChenAttribute, EntityImageChenCircle, EntityImageChenEntity,
    EntityImageChenRelationship,
};
pub(crate) use entity_image_note::EntityImageNote;
#[cfg(test)]
pub(crate) use entity_image_note::OpaleLink;
pub(crate) use entity_image_note_link::EntityImageNoteLink;
pub(crate) use entity_image_tips::EntityImageTips;
pub(crate) use opale::{get_corner, get_polygon_normal};
