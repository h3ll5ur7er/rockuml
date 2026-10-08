//! The tiles of compound instructions and the factories building them (PlantUML's
//! `activitydiagram3.ftile.vcompact`).

mod cond;
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
mod ftile_if_down;
mod ftile_if_long_horizontal;
mod ftile_if_long_vertical;
mod ftile_repeat;
mod ftile_while;
pub(crate) mod one_swimlane;
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
pub(crate) use ftile_if_down::FtileIfDown;
pub(crate) use ftile_if_long_horizontal::FtileIfLongHorizontal;
pub(crate) use ftile_if_long_vertical::FtileIfLongVertical;
pub(crate) use v_compact_factory::VCompactFactory;

use std::rc::Rc;

use super::FtileFactory;
use crate::creole::{CreoleMode, Display};
use crate::klimt::font::FontConfiguration;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;

/// `display` drawn as `Display.create0` draws it; a block taking no room for PlantUML's `Display.NULL`,
/// which draws nothing.
pub(crate) fn create0_or_empty(
    display: Option<&Display>,
    font: &FontConfiguration,
    alignment: HorizontalAlignment,
    skin_param: &SkinParam,
    max_width: f64,
    mode: CreoleMode,
) -> Rc<dyn TextBlock> {
    match display {
        Some(display) => Rc::new(display.create0(font, alignment, skin_param, max_width, mode)),
        None => super::vertical::empty_label(),
    }
}

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

#[cfg(test)]
mod tests;
