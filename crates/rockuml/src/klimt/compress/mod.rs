//! Activity diagrams squeeze the empty space out of their drawing, first across, then down (PlantUML's
//! `klimt.compress`).
//!
//! A [`CompressionXorYBuilder`] draws its block once on a [`slot_finder::SlotFinder`], a measuring surface
//! recording where along the axis shapes take room. The free space between those slots, less a margin on each
//! side, becomes a [`compression_transform::CompressionTransform`], and the block is drawn through an
//! [`ugraphic_compress_on_x_or_y::UGraphicCompressOnXorY`] layer that moves every position back by the free
//! space before it.
#![allow(
    dead_code,
    unused_imports,
    reason = "ActivityDiagram3::text_block compresses its swimlanes once activity diagrams draw, Phase 6 stage B"
)]

mod compression_transform;
mod compression_x_or_y_builder;
mod slot;
mod slot_finder;
mod ugraphic_compress_on_x_or_y;

pub(crate) use compression_x_or_y_builder::CompressionXorYBuilder;

/// The axis a compression pass works on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CompressionMode {
    OnX,
    OnY,
}

#[cfg(test)]
mod tests;
