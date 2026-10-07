//! PlantUML's `FtileFactoryDelegatorCreateParallel`: overrides `createParallel`, for fork, merge and
//! split. Until that is ported it passes everything on.

use crate::ftile::{FtileFactory, FtileFactoryDelegator};

pub(crate) struct FtileFactoryDelegatorCreateParallel {
    factory: Box<dyn FtileFactory>,
}

impl FtileFactoryDelegatorCreateParallel {
    pub(crate) fn new(factory: Box<dyn FtileFactory>) -> Self {
        Self { factory }
    }
}

impl FtileFactoryDelegator for FtileFactoryDelegatorCreateParallel {
    fn get_factory(&self) -> &dyn FtileFactory {
        self.factory.as_ref()
    }
}
