//! PlantUML's `FtileFactoryDelegatorAddUrl`: overrides `addUrl`, which links boxes. Until that is ported
//! it passes everything on.

use crate::ftile::{FtileFactory, FtileFactoryDelegator};

pub(crate) struct FtileFactoryDelegatorAddUrl {
    factory: Box<dyn FtileFactory>,
}

impl FtileFactoryDelegatorAddUrl {
    pub(crate) fn new(factory: Box<dyn FtileFactory>) -> Self {
        Self { factory }
    }
}

impl FtileFactoryDelegator for FtileFactoryDelegatorAddUrl {
    fn get_factory(&self) -> &dyn FtileFactory {
        self.factory.as_ref()
    }
}
