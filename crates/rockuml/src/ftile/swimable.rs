//! What lies in swimlanes: tiles, and the instructions they are built from (PlantUML's `Swimable`).

use crate::diagram::activity3::{SwimlaneId, SwimlaneSet};

/// `None` stands for PlantUML's `null` swimlane, as tiles of diagrams without swimlanes have.
pub(crate) trait Swimable {
    /// The lanes the thing lies in; like PlantUML's set it may hold `None`. It is only asked about
    /// membership and size, and iterated when it holds one lane, so its order does not show.
    fn get_swimlanes(&self) -> SwimlaneSet;

    /// The lane arrows enter it in.
    fn get_swimlane_in(&self) -> Option<SwimlaneId>;

    /// The lane arrows leave it from.
    fn get_swimlane_out(&self) -> Option<SwimlaneId>;
}
