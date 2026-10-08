//! The tiles of `if` and `switch` (PlantUML's `activitydiagram3.ftile.vcompact.cond`).

mod conditional_builder;
mod ftile_if_with_links;
mod ftile_switch_with_diamonds;
pub(super) mod ftile_switch_with_many_links;
pub(super) mod ftile_switch_with_one_link;

pub(crate) use conditional_builder::ConditionalBuilder;
pub(crate) use ftile_if_with_links::FtileIfWithLinks;
