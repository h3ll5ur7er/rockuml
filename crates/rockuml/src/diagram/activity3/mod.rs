//! Activity diagrams, new syntax (PlantUML's `activitydiagram3`).

mod link_rendering;
mod swimlane;

pub(crate) use link_rendering::LinkRendering;
pub(crate) use swimlane::{Swimlane, SwimlaneId};
