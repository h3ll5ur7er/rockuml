//! What connections share (PlantUML's `AbstractConnection`): the two tiles they join.

use std::rc::Rc;

use super::Ftile;
use crate::klimt::HorizontalAlignment;

/// Embedded in a connection, which forwards [`super::Connection::get_ftile1`] and
/// [`super::Connection::get_ftile2`] to it.
pub(crate) struct AbstractConnection {
    ftile1: Option<Rc<dyn Ftile>>,
    ftile2: Option<Rc<dyn Ftile>>,
}

impl AbstractConnection {
    pub(crate) fn new(ftile1: Option<Rc<dyn Ftile>>, ftile2: Option<Rc<dyn Ftile>>) -> Self {
        Self { ftile1, ftile2 }
    }

    pub(crate) fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.ftile1.as_ref()
    }

    pub(crate) fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.ftile2.as_ref()
    }

    /// Where the labels of the arrow go: as the tile it leaves, or else the one it enters, says.
    pub(crate) fn arrow_horizontal_alignment(&self) -> HorizontalAlignment {
        self.ftile1
            .as_ref()
            .or(self.ftile2.as_ref())
            .map_or(HorizontalAlignment::Left, |tile| {
                tile.arrow_horizontal_alignment()
            })
    }
}
