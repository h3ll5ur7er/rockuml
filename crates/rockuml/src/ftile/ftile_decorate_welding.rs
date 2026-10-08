//! A tile holding `break`s, which the loop around it joins to its exit (PlantUML's `FtileDecorateWelding`).

use std::rc::Rc;

use super::Ftile;
use super::vertical::FtileDecorate;

pub(crate) struct FtileDecorateWelding {
    ftile: Rc<dyn Ftile>,
    breaks: Vec<Rc<dyn Ftile>>,
}

impl FtileDecorateWelding {
    pub(crate) fn new(ftile: Rc<dyn Ftile>, breaks: Vec<Rc<dyn Ftile>>) -> Self {
        Self { ftile, breaks }
    }
}

impl FtileDecorate for FtileDecorateWelding {
    fn get_ftile_delegated(&self) -> &Rc<dyn Ftile> {
        &self.ftile
    }

    fn get_welding_points(&self) -> Vec<Rc<dyn Ftile>> {
        self.breaks.clone()
    }
}
