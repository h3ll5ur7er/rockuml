//! PlantUML's `FtileHeightFixedMarged`: a tile with room above and below it, where the flows of a fork
//! keep the labels of their arrows in and out.

use std::cell::OnceCell;
use std::rc::Rc;

use super::{Ftile, FtileGeometry, Swimable};
use crate::diagram::activity3::{LinkRendering, SwimlaneId, SwimlaneSet};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::UTranslate;
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;

pub(crate) struct FtileHeightFixedMarged {
    tile: Rc<dyn Ftile>,
    ymargin1: f64,
    ymargin2: f64,
    cached_geometry: OnceCell<FtileGeometry>,
}

impl FtileHeightFixedMarged {
    pub(crate) fn new(ymargin1: f64, tile: Rc<dyn Ftile>, ymargin2: f64) -> Self {
        Self {
            tile,
            ymargin1,
            ymargin2,
            cached_geometry: OnceCell::new(),
        }
    }

    fn get_translate(&self) -> UTranslate {
        UTranslate::new(0.0, self.ymargin1)
    }
}

impl Swimable for FtileHeightFixedMarged {
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

impl Ftile for FtileHeightFixedMarged {
    fn skin_param(&self) -> &SkinParam {
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
            let dim = self.tile.calculate_dimension(string_bounder);
            dim.translate(self.get_translate())
                .fixed_height(self.ymargin1 + dim.get_height() + self.ymargin2)
        })
    }

    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        vec![self.tile.clone()]
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.apply(self.get_translate()).draw(&self.tile);
    }
}
