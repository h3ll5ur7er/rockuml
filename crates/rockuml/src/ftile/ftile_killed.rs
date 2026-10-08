//! A tile after which the flow stops: `kill` or `detach` (PlantUML's `FtileKilled`).

use std::cell::OnceCell;
use std::rc::Rc;

use super::{Ftile, FtileGeometry, Swimable};
use crate::diagram::activity3::{SwimlaneId, SwimlaneSet};
use crate::klimt::font::StringBounder;
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;

pub(crate) struct FtileKilled {
    tile: Rc<dyn Ftile>,
    cached_geometry: OnceCell<FtileGeometry>,
}

impl FtileKilled {
    pub(crate) fn new(tile_to_kill: Rc<dyn Ftile>) -> Self {
        Self {
            tile: tile_to_kill,
            cached_geometry: OnceCell::new(),
        }
    }
}

impl Swimable for FtileKilled {
    fn get_swimlanes(&self) -> SwimlaneSet {
        self.tile.get_swimlanes()
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.tile.get_swimlane_in()
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.tile.get_swimlane_out()
    }
}

impl Ftile for FtileKilled {
    fn skin_param(&self) -> &Rc<SkinParam> {
        self.tile.skin_param()
    }

    /// The killed tile's geometry without its point out.
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        *self.cached_geometry.get_or_init(|| {
            let geo = self.tile.calculate_dimension(string_bounder);
            FtileGeometry::from_dim(geo.dimension(), geo.get_left(), geo.get_in_y())
        })
    }

    /// The killed tile's children, as PlantUML lists them; the killed tile itself is not one.
    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        self.tile.get_my_children()
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.draw(&self.tile);
    }
}
