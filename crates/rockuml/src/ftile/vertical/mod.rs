//! The tiles of single instructions (PlantUML's `activitydiagram3.ftile.vertical`).

mod ftile_diamond;
mod ftile_diamond_inside;
mod ftile_diamond_inside2;
mod ftile_diamond_square;
mod ftile_diamond_wip;

pub(crate) use ftile_diamond::FtileDiamond;
pub(crate) use ftile_diamond_inside::FtileDiamondInside;
pub(crate) use ftile_diamond_inside2::FtileDiamondInside2;
pub(crate) use ftile_diamond_square::FtileDiamondSquare;
pub(crate) use ftile_diamond_wip::empty_label;
