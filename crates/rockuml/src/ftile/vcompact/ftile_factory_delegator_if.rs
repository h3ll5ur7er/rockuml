//! PlantUML's `FtileFactoryDelegatorIf`: overrides `createIf`, which lays out an `if` with
//! `ConditionalBuilder`, or `FtileIfLongHorizontal` / `FtileIfLongVertical` for `elseif`s. It hands those
//! builders [`FtileFactoryDelegator::get_factory`], the chain inside it. Until that is ported it passes
//! everything on.

use crate::ftile::{FtileFactory, FtileFactoryDelegator};

pub(crate) struct FtileFactoryDelegatorIf {
    factory: Box<dyn FtileFactory>,
    /// `!pragma useVerticalIf on`: `elseif`s go down rather than across.
    use_vertical_if: bool,
}

impl FtileFactoryDelegatorIf {
    pub(crate) fn new(factory: Box<dyn FtileFactory>, use_vertical_if: bool) -> Self {
        Self {
            factory,
            use_vertical_if,
        }
    }
}

impl FtileFactoryDelegator for FtileFactoryDelegatorIf {
    fn get_factory(&self) -> &dyn FtileFactory {
        self.factory.as_ref()
    }
}
