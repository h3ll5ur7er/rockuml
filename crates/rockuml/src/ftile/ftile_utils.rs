//! Wrapping tiles (PlantUML's `FtileUtils`).

use std::rc::Rc;

use super::{Connection, Ftile, FtileMarged, FtileWithConnection};
use crate::skin::SkinParam;

/// `ftile` with `connections` drawn over it; `ftile` itself without any.
pub(crate) fn add_connection(
    ftile: Rc<dyn Ftile>,
    connections: Vec<Rc<dyn Connection>>,
) -> Rc<dyn Ftile> {
    if connections.is_empty() {
        return ftile;
    }
    Rc::new(FtileWithConnection::new(ftile, connections))
}

/// `ftile` with `margin1` left of it and `margin2` right of it; `skin_param` is the one it is drawn with.
pub(crate) fn add_horizontal_margin(
    ftile: Rc<dyn Ftile>,
    skin_param: Rc<SkinParam>,
    margin1: f64,
    margin2: f64,
) -> Rc<dyn Ftile> {
    if margin1 == 0.0 && margin2 == 0.0 {
        return ftile;
    }
    Rc::new(FtileMarged::new(ftile, skin_param, margin1, margin2))
}
