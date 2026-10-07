//! What lies in swimlanes: tiles, and the instructions they are built from (PlantUML's `Swimable`).

use std::collections::BTreeSet;

use crate::diagram::activity3::SwimlaneId;

/// `None` stands for PlantUML's `null` swimlane, as tiles of diagrams without swimlanes have.
pub(crate) trait Swimable {
    /// The lanes the thing lies in. PlantUML's set is only ever asked about membership and size, and
    /// iterated when it holds one lane, so the order does not show.
    fn get_swimlanes(&self) -> BTreeSet<SwimlaneId>;

    /// The lane arrows enter it in.
    fn get_swimlane_in(&self) -> Option<SwimlaneId>;

    /// The lane arrows leave it from.
    fn get_swimlane_out(&self) -> Option<SwimlaneId>;
}
