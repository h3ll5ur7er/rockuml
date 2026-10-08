//! A tile with arrows drawn after it (PlantUML's `FtileWithConnection`).

use std::rc::Rc;

use super::vertical::FtileDecorate;
use super::{Connection, Ftile};
use crate::klimt::ugraphic::UGraphic;

pub(crate) struct FtileWithConnection {
    ftile: Rc<dyn Ftile>,
    connections: Vec<Rc<dyn Connection>>,
}

impl FtileWithConnection {
    /// `connections` is not empty.
    pub(crate) fn new(ftile: Rc<dyn Ftile>, connections: Vec<Rc<dyn Connection>>) -> Self {
        Self { ftile, connections }
    }
}

impl FtileDecorate for FtileWithConnection {
    fn get_ftile_delegated(&self) -> &Rc<dyn Ftile> {
        &self.ftile
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.ftile.draw_u(ug);
        for connection in &self.connections {
            ug.draw(connection);
        }
    }
}
