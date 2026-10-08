//! A tile entered or left in another lane than its own say (PlantUML's `FtileWithSwimlanes`).

use std::rc::Rc;

use super::Ftile;
use super::vertical::FtileDecorate;
use crate::diagram::activity3::SwimlaneId;

pub(crate) struct FtileWithSwimlanes {
    ftile: Rc<dyn Ftile>,
    in_: Option<SwimlaneId>,
    out: Option<SwimlaneId>,
}

impl FtileWithSwimlanes {
    pub(crate) fn new(
        ftile: Rc<dyn Ftile>,
        in_: Option<SwimlaneId>,
        out: Option<SwimlaneId>,
    ) -> Self {
        Self { ftile, in_, out }
    }
}

impl FtileDecorate for FtileWithSwimlanes {
    fn get_ftile_delegated(&self) -> &Rc<dyn Ftile> {
        &self.ftile
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.in_
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.out
    }
}
