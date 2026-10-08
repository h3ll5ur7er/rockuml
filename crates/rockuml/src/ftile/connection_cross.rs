//! An arrow from one swimlane into another (PlantUML's `ConnectionCross`).

use super::Connection;
use crate::diagram::activity3::SwimlaneId;
use crate::klimt::geom::UTranslate;
use crate::klimt::ugraphic::UGraphic;

pub(crate) struct ConnectionCross<'a> {
    connection: &'a dyn Connection,
}

impl<'a> ConnectionCross<'a> {
    pub(crate) fn new(connection: &'a dyn Connection) -> Self {
        Self { connection }
    }

    /// Draws the connection with its start moved to the lane it leaves and its end to the lane it enters,
    /// `get_translate` telling where each lane is; connections that cannot be drawn so, or whose tiles lie
    /// in no lane, are not drawn.
    pub(crate) fn draw_u(&self, ug: &UGraphic, get_translate: impl Fn(SwimlaneId) -> UTranslate) {
        let Some(conn) = self.connection.as_translatable() else {
            return;
        };
        let swimlane1 = self
            .connection
            .get_ftile1()
            .and_then(|tile1| tile1.get_swimlane_out());
        let swimlane2 = self
            .connection
            .get_ftile2()
            .and_then(|tile2| tile2.get_swimlane_in());
        let (Some(swimlane1), Some(swimlane2)) = (swimlane1, swimlane2) else {
            return;
        };
        conn.draw_translate(ug, get_translate(swimlane1), get_translate(swimlane2));
    }
}
