//! The arrows a tile draws between its children (PlantUML's `Connection` and `ConnectionTranslatable`).

use std::rc::Rc;

use super::Ftile;
use crate::klimt::geom::UTranslate;
use crate::klimt::ugraphic::UGraphic;

/// Drawn with `ug.draw(&connection)`, so that swimlane layers can draw it only in the lanes of its tiles.
/// Connections are values built anew each time a tile draws or lists them; the tiles they join are the
/// tile's own children, so that identity (`ftile::same`) finds them.
pub(crate) trait Connection {
    /// The tile the arrow leaves, if any.
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>>;

    /// The tile the arrow enters, if any.
    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>>;

    fn draw_u(&self, ug: &UGraphic);

    /// The connection as one that can be drawn across swimlanes (PlantUML's `instanceof
    /// ConnectionTranslatable`).
    fn as_translatable(&self) -> Option<&dyn ConnectionTranslatable> {
        None
    }
}

/// A connection that can be drawn between two swimlanes, its start moved by `translate1` (the lane of
/// `ftile1`) and its end by `translate2`; `ConnectionCross` draws it so.
pub(crate) trait ConnectionTranslatable: Connection {
    fn draw_translate(&self, ug: &UGraphic, translate1: UTranslate, translate2: UTranslate);
}
