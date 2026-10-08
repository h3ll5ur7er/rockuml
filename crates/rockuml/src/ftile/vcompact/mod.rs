//! The tiles of compound instructions and the factories building them (PlantUML's
//! `activitydiagram3.ftile.vcompact`).

mod ftile_factory_delegator_add_note;
mod ftile_factory_delegator_add_url;
mod ftile_factory_delegator_assembly;
mod ftile_factory_delegator_create_group;
mod ftile_factory_delegator_create_parallel;
mod ftile_factory_delegator_if;
mod ftile_factory_delegator_repeat;
mod ftile_factory_delegator_switch;
mod ftile_factory_delegator_while;

pub(crate) use ftile_factory_delegator_add_note::FtileFactoryDelegatorAddNote;
pub(crate) use ftile_factory_delegator_add_url::FtileFactoryDelegatorAddUrl;
pub(crate) use ftile_factory_delegator_assembly::FtileFactoryDelegatorAssembly;
pub(crate) use ftile_factory_delegator_create_group::FtileFactoryDelegatorCreateGroup;
pub(crate) use ftile_factory_delegator_create_parallel::FtileFactoryDelegatorCreateParallel;
pub(crate) use ftile_factory_delegator_if::FtileFactoryDelegatorIf;
pub(crate) use ftile_factory_delegator_repeat::FtileFactoryDelegatorRepeat;
pub(crate) use ftile_factory_delegator_switch::FtileFactoryDelegatorSwitch;
pub(crate) use ftile_factory_delegator_while::FtileFactoryDelegatorWhile;

use super::FtileFactory;

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
