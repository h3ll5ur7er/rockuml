//! PlantUML's `FtileFactoryDelegatorAddNote`: overrides `addNote`. Until that is ported it passes
//! everything on.

use crate::ftile::{FtileFactory, FtileFactoryDelegator};

pub(crate) struct FtileFactoryDelegatorAddNote {
    factory: Box<dyn FtileFactory>,
}

impl FtileFactoryDelegatorAddNote {
    pub(crate) fn new(factory: Box<dyn FtileFactory>) -> Self {
        Self { factory }
    }
}

impl FtileFactoryDelegator for FtileFactoryDelegatorAddNote {
    fn get_factory(&self) -> &dyn FtileFactory {
        self.factory.as_ref()
    }
}
