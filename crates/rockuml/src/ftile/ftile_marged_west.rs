//! A tile with room on its left (PlantUML's `FtileMargedWest`), for the labels of the arrows into the
//! conditions of a vertical `elseif` chain.

use std::rc::Rc;

use super::vertical::FtileDecorate;
use super::{Ftile, FtileGeometry};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::UTranslate;
use crate::klimt::ugraphic::UGraphic;

/// PlantUML draws the tile `margin` further right, yet says it is drawn where this one is.
pub(crate) struct FtileMargedWest {
    tile: Rc<dyn Ftile>,
    margin: f64,
}

impl FtileMargedWest {
    pub(crate) fn new(tile: Rc<dyn Ftile>, margin: f64) -> Self {
        Self { tile, margin }
    }
}

impl FtileDecorate for FtileMargedWest {
    fn get_ftile_delegated(&self) -> &Rc<dyn Ftile> {
        &self.tile
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.tile
            .draw_u(&ug.apply(UTranslate::new(self.margin, 0.0)));
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.tile
            .calculate_dimension(string_bounder)
            .inc_left(self.margin)
    }
}
