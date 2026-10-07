//! What PlantUML's `svek` package holds for laying out and drawing entity diagrams.

pub(crate) mod extremity;

mod entity_image;
pub(crate) mod image;
mod margins;
mod rounded_container;
mod rounded_north;
mod rounded_south;
mod shape_type;

#[cfg_attr(test, allow(unused_imports, reason = "drawn by the Smetana bridge"))]
pub(crate) use entity_image::{AbstractEntityImage, IEntityImage, MARGIN, MARGIN_LINE};
pub(crate) use margins::Margins;
pub(crate) use shape_type::ShapeType;
