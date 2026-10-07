//! What PlantUML's `svek` package holds for laying out and drawing entity diagrams.

pub(crate) mod extremity;
pub(crate) mod image;

mod entity_image;
mod margins;
mod shape_type;

#[cfg_attr(test, allow(unused_imports, reason = "drawn by the Smetana bridge"))]
pub(crate) use entity_image::{AbstractEntityImage, IEntityImage};
pub(crate) use margins::Margins;
pub(crate) use shape_type::ShapeType;
