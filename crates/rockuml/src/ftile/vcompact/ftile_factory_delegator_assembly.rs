//! PlantUML's `FtileFactoryDelegatorAssembly`: overrides `assembly`, which joins two tiles with an arrow
//! and room for its label. Until that is ported it passes everything on.

use crate::ftile::{FtileFactory, FtileFactoryDelegator};

pub(crate) struct FtileFactoryDelegatorAssembly {
    factory: Box<dyn FtileFactory>,
}

impl FtileFactoryDelegatorAssembly {
    pub(crate) fn new(factory: Box<dyn FtileFactory>) -> Self {
        Self { factory }
    }
}

impl FtileFactoryDelegator for FtileFactoryDelegatorAssembly {
    fn get_factory(&self) -> &dyn FtileFactory {
        self.factory.as_ref()
    }
}
