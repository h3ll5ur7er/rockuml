//! PlantUML's `FtileHeightFixedCentered`: a tile centred down a fixed height, as the flows of a fork are,
//! so that they line up between its bars.

use std::cell::OnceCell;
use std::rc::Rc;

use super::{Ftile, FtileGeometry, Swimable};
use crate::diagram::activity3::{LinkRendering, SwimlaneId, SwimlaneSet};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::UTranslate;
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;

pub(crate) struct FtileHeightFixedCentered {
    tile: Rc<dyn Ftile>,
    fixed_height: f64,
    cached_geometry: OnceCell<FtileGeometry>,
}

impl FtileHeightFixedCentered {
    /// `fixed_height` is at least the height of `tile`.
    pub(crate) fn new(tile: Rc<dyn Ftile>, fixed_height: f64) -> Self {
        Self {
            tile,
            fixed_height,
            cached_geometry: OnceCell::new(),
        }
    }

    fn get_translate(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim = self.tile.calculate_dimension(string_bounder);
        UTranslate::new(0.0, (self.fixed_height - dim.get_height()) / 2.0)
    }
}

impl Swimable for FtileHeightFixedCentered {
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

impl Ftile for FtileHeightFixedCentered {
    fn skin_param(&self) -> &Rc<SkinParam> {
        self.tile.skin_param()
    }

    fn get_in_link_rendering(&self) -> LinkRendering {
        self.tile.get_in_link_rendering()
    }

    fn get_out_link_rendering(&self) -> LinkRendering {
        self.tile.get_out_link_rendering()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        *self.cached_geometry.get_or_init(|| {
            self.tile
                .calculate_dimension(string_bounder)
                .translate(self.get_translate(string_bounder))
                .fixed_height(self.fixed_height)
        })
    }

    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        vec![self.tile.clone()]
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.apply(self.get_translate(ug.string_bounder()))
            .draw(&self.tile);
    }
}
