//! PlantUML's `FtileFactoryDelegatorRepeat`: overrides `repeat`. Until that is ported it passes
//! everything on.

use crate::ftile::{FtileFactory, FtileFactoryDelegator};

pub(crate) struct FtileFactoryDelegatorRepeat {
    factory: Box<dyn FtileFactory>,
}

impl FtileFactoryDelegatorRepeat {
    pub(crate) fn new(factory: Box<dyn FtileFactory>) -> Self {
        Self { factory }
    }
}

impl FtileFactoryDelegator for FtileFactoryDelegatorRepeat {
    fn get_factory(&self) -> &dyn FtileFactory {
        self.factory.as_ref()
    }
}
