//! What PlantUML's `svek` package holds for laying out and drawing entity diagrams.

pub(crate) mod extremity;

mod bibliotekon;
mod cluster;
mod cluster_decoration;
mod cluster_header;
mod cluster_manager;
mod color_sequence;
mod entity_image;
mod general_image_builder;
mod margins;
mod shape_type;
mod svek_node;

pub(crate) use bibliotekon::Bibliotekon;
pub(crate) use cluster::{Cluster, ClusterId};
use cluster_decoration::ClusterDecoration;
pub(crate) use cluster_header::ClusterHeader;
pub(crate) use cluster_manager::ClusterManager;
use color_sequence::ColorSequence;
pub(crate) use entity_image::{AbstractEntityImage, IEntityImage, LayoutContext};
pub(crate) use general_image_builder::create_entity_image_block;
pub(crate) use margins::Margins;
pub(crate) use shape_type::ShapeType;
pub(crate) use svek_node::SvekNode;

/// Room around the text of entity images (`IEntityImage.MARGIN`).
pub(crate) const MARGIN: i32 = 5;
