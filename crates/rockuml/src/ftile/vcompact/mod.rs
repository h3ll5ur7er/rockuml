//! The tiles of compound instructions and the factories building them (PlantUML's
//! `activitydiagram3.ftile.vcompact`).

mod abstract_parallel_ftiles_builder;
mod connection_vertical_down;
mod ftile_factory_delegator_add_note;
mod ftile_factory_delegator_add_url;
mod ftile_factory_delegator_assembly;
mod ftile_factory_delegator_create_group;
mod ftile_factory_delegator_create_parallel;
mod ftile_factory_delegator_if;
mod ftile_factory_delegator_repeat;
mod ftile_factory_delegator_switch;
mod ftile_factory_delegator_while;
mod ftile_fork_inner;
mod ftile_group;
mod ftile_note_alone;
mod ftile_repeat;
mod ftile_while;
mod ftile_with_note_opale;
mod ftile_with_notes;
mod note_sheet;
mod parallel_builder_fork;
mod parallel_builder_merge;
mod parallel_builder_split;
mod v_compact_factory;

pub(crate) use connection_vertical_down::ConnectionVerticalDown;
pub(crate) use ftile_factory_delegator_add_note::FtileFactoryDelegatorAddNote;
pub(crate) use ftile_factory_delegator_add_url::FtileFactoryDelegatorAddUrl;
pub(crate) use ftile_factory_delegator_assembly::FtileFactoryDelegatorAssembly;
pub(crate) use ftile_factory_delegator_create_group::FtileFactoryDelegatorCreateGroup;
pub(crate) use ftile_factory_delegator_create_parallel::FtileFactoryDelegatorCreateParallel;
pub(crate) use ftile_factory_delegator_if::FtileFactoryDelegatorIf;
pub(crate) use ftile_factory_delegator_repeat::FtileFactoryDelegatorRepeat;
pub(crate) use ftile_factory_delegator_switch::FtileFactoryDelegatorSwitch;
pub(crate) use ftile_factory_delegator_while::FtileFactoryDelegatorWhile;
pub(crate) use ftile_fork_inner::FtileForkInner;
pub(crate) use ftile_group::FtileGroup;
pub(crate) use ftile_note_alone::FtileNoteAlone;
pub(crate) use ftile_with_note_opale::FtileWithNoteOpale;
pub(crate) use ftile_with_notes::FtileWithNotes;
pub(crate) use v_compact_factory::VCompactFactory;

use std::rc::Rc;

use super::FtileFactory;
use crate::creole::{CreoleMode, Display};
use crate::klimt::font::FontConfiguration;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;

/// The factory instructions build with: `v_compact_factory` wrapped in PlantUML's delegators, innermost
/// first (`Swimlanes.getFtileFactory`). `use_vertical_if` is `!pragma useVerticalIf`.
pub(crate) fn delegator_chain(
    v_compact_factory: Box<dyn FtileFactory>,
    use_vertical_if: bool,
) -> Box<dyn FtileFactory> {
    let factory = Box::new(FtileFactoryDelegatorAddUrl::new(v_compact_factory));
    let factory = Box::new(FtileFactoryDelegatorAssembly::new(factory));
    let factory = Box::new(FtileFactoryDelegatorIf::new(factory, use_vertical_if));
    let factory = Box::new(FtileFactoryDelegatorSwitch::new(factory));
    let factory = Box::new(FtileFactoryDelegatorWhile::new(factory));
    let factory = Box::new(FtileFactoryDelegatorRepeat::new(factory));
    let factory = Box::new(FtileFactoryDelegatorCreateParallel::new(factory));
    let factory = Box::new(FtileFactoryDelegatorAddNote::new(factory));
    Box::new(FtileFactoryDelegatorCreateGroup::new(factory))
}

/// `display` drawn in `font`; PlantUML's `Display.NULL` (`None`) draws as no lines at all.
fn display_text(
    display: Option<&Display>,
    font: &FontConfiguration,
    alignment: HorizontalAlignment,
    skin_param: &SkinParam,
    mode: CreoleMode,
) -> Rc<dyn TextBlock> {
    let no_lines = Display::default();
    Rc::new(
        display
            .unwrap_or(&no_lines)
            .create0(font, alignment, skin_param, 0.0, mode),
    )
}

#[cfg(test)]
mod tests;
