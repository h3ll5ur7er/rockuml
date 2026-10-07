//! PlantUML's `FtileFactoryDelegatorWhile`: overrides `createWhile`. Until that is ported it passes
//! everything on.

use crate::ftile::{FtileFactory, FtileFactoryDelegator};

pub(crate) struct FtileFactoryDelegatorWhile {
    factory: Box<dyn FtileFactory>,
}

impl FtileFactoryDelegatorWhile {
    pub(crate) fn new(factory: Box<dyn FtileFactory>) -> Self {
        Self { factory }
    }
}

impl FtileFactoryDelegator for FtileFactoryDelegatorWhile {
    fn get_factory(&self) -> &dyn FtileFactory {
        self.factory.as_ref()
    }
}
