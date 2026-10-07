//! What PlantUML's `svek` package holds for laying out and drawing entity diagrams.

pub(crate) mod extremity;

mod bibliotekon;
mod cluster;
mod cluster_decoration;
mod cluster_header;
mod cluster_manager;
mod color_sequence;
mod concurrent_states;
mod entity_image;
mod general_image_builder;
pub(crate) mod image;
mod inner_state_autonom;
mod margins;
mod rounded_container;
mod rounded_north;
mod rounded_south;
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

/// Room between a separator line and the text next to it (`IEntityImage.MARGIN_LINE`).
pub(crate) const MARGIN_LINE: i32 = 5;
