//! Wrapping tiles (PlantUML's `FtileUtils`).

use std::rc::Rc;

use super::{
    Connection, Ftile, FtileMarged, FtileMargedVertically, FtileWithConnection, FtileWithSwimlanes,
};
use crate::diagram::activity3::SwimlaneId;

pub(crate) fn add_connection(
    ftile: Rc<dyn Ftile>,
    connection: Rc<dyn Connection>,
) -> Rc<dyn Ftile> {
    Rc::new(FtileWithConnection::new(ftile, vec![connection]))
}

/// The tile itself when there are no connections.
pub(crate) fn add_connections(
    ftile: Rc<dyn Ftile>,
    connections: Vec<Rc<dyn Connection>>,
) -> Rc<dyn Ftile> {
    if connections.is_empty() {
        return ftile;
    }
    Rc::new(FtileWithConnection::new(ftile, connections))
}

pub(crate) fn with_swimlane_in(ftile: Rc<dyn Ftile>, in_: Option<SwimlaneId>) -> Rc<dyn Ftile> {
    let out = ftile.get_swimlane_out();
    Rc::new(FtileWithSwimlanes::new(ftile, in_, out))
}

pub(crate) fn add_bottom(ftile: Rc<dyn Ftile>, margin_bottom: f64) -> Rc<dyn Ftile> {
    Rc::new(FtileMargedVertically::new(ftile, 0.0, margin_bottom))
}

pub(crate) fn add_vertical_margin(
    ftile: Rc<dyn Ftile>,
    margin_top: f64,
    margin_bottom: f64,
) -> Rc<dyn Ftile> {
    if margin_top == 0.0 && margin_bottom == 0.0 {
        return ftile;
    }
    Rc::new(FtileMargedVertically::new(ftile, margin_top, margin_bottom))
}

pub(crate) fn add_horizontal_margin(
    ftile: Rc<dyn Ftile>,
    margin1: f64,
    margin2: f64,
) -> Rc<dyn Ftile> {
    if margin1 == 0.0 && margin2 == 0.0 {
        return ftile;
    }
    Rc::new(FtileMarged::new(ftile, margin1, margin2))
}
