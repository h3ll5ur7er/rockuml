//! PlantUML's `FtileFactoryDelegatorSwitch`: overrides `createSwitch`. Until that is ported it passes
//! everything on.

use crate::ftile::{FtileFactory, FtileFactoryDelegator};

pub(crate) struct FtileFactoryDelegatorSwitch {
    factory: Box<dyn FtileFactory>,
}

impl FtileFactoryDelegatorSwitch {
    pub(crate) fn new(factory: Box<dyn FtileFactory>) -> Self {
        Self { factory }
    }
}

impl FtileFactoryDelegator for FtileFactoryDelegatorSwitch {
    fn get_factory(&self) -> &dyn FtileFactory {
        self.factory.as_ref()
    }
}
