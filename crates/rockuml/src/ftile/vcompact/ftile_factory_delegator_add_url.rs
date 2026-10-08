//! PlantUML's `FtileFactoryDelegatorAddUrl`: links boxes.

use std::rc::Rc;

use crate::ftile::vertical::{FtileBox, FtileBoxEmoji};
use crate::ftile::{Ftile, FtileFactory, FtileFactoryDelegator, FtileWithUrl, downcast};
use crate::klimt::url::Url;

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

    /// Only boxes, and icons, take links.
    fn add_url(&self, ftile: Rc<dyn Ftile>, url: &Url) -> Rc<dyn Ftile> {
        if downcast::<FtileBox>(ftile.as_ref()).is_some()
            || downcast::<FtileBoxEmoji>(ftile.as_ref()).is_some()
        {
            return Rc::new(FtileWithUrl::new(ftile, url.clone()));
        }
        ftile
    }
}
