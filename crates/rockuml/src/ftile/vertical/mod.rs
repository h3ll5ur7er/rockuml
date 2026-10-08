//! The tiles of single instructions, and the decorators the factories wrap tiles in (PlantUML's
//! `activitydiagram3.ftile.vertical`).

mod ftile_box;
mod ftile_circles;
mod ftile_decorate;
mod ftile_diamond;
mod ftile_diamond_inside;
mod ftile_diamond_inside2;
mod ftile_diamond_square;
mod ftile_diamond_wip;

pub(crate) use ftile_box::FtileBox;
pub(crate) use ftile_circles::{
    FtileCircleEndCross, FtileCircleSpot, FtileCircleStart, FtileCircleStop,
};
pub(crate) use ftile_decorate::{
    FtileDecorate, FtileDecorateIn, FtileDecorateInLabel, FtileDecorateOut, FtileDecorateOutLabel,
};
pub(crate) use ftile_diamond::FtileDiamond;
pub(crate) use ftile_diamond_inside::FtileDiamondInside;
pub(crate) use ftile_diamond_inside2::FtileDiamondInside2;
pub(crate) use ftile_diamond_square::FtileDiamondSquare;
pub(crate) use ftile_diamond_wip::empty_label;
