//! PlantUML's `FtileFactoryDelegatorCreateGroup`: overrides `createGroup`. Until that is ported it passes
//! everything on.

use crate::ftile::{FtileFactory, FtileFactoryDelegator};

pub(crate) struct FtileFactoryDelegatorCreateGroup {
    factory: Box<dyn FtileFactory>,
}

impl FtileFactoryDelegatorCreateGroup {
    pub(crate) fn new(factory: Box<dyn FtileFactory>) -> Self {
        Self { factory }
    }
}

impl FtileFactoryDelegator for FtileFactoryDelegatorCreateGroup {
    fn get_factory(&self) -> &dyn FtileFactory {
        self.factory.as_ref()
    }
}
