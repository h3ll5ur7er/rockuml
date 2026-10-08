//! A tile drawing nothing (PlantUML's `FtileEmpty`).

use std::rc::Rc;

use super::{AbstractFtile, Ftile, FtileGeometry, Swimable};
use crate::diagram::activity3::{SwimlaneId, SwimlaneSet};
use crate::klimt::font::StringBounder;
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;

pub(crate) struct FtileEmpty {
    base: AbstractFtile,
    swimlane: Option<SwimlaneId>,
}

impl FtileEmpty {
    /// Taking no room.
    pub(crate) fn new(skin_param: Rc<SkinParam>, swimlane: Option<SwimlaneId>) -> Self {
        Self {
            base: AbstractFtile::new(skin_param),
            swimlane,
        }
    }
}

impl Swimable for FtileEmpty {
    fn get_swimlanes(&self) -> SwimlaneSet {
        self.swimlane.map(Some).into_iter().collect()
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.swimlane
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.swimlane
    }
}

impl Ftile for FtileEmpty {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base
            .calculate_dimension(|| FtileGeometry::with_out(0.0, 0.0, 0.0, 0.0, 0.0))
    }

    fn draw_u(&self, _ug: &UGraphic) {}
}
