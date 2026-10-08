//! PlantUML's `FtileThinSplit`: the thin line a split starts and ends at, from the first flow to the
//! last.

use std::rc::Rc;

use crate::color::HColor;
use crate::diagram::activity3::{SwimlaneId, SwimlaneSet};
use crate::ftile::{AbstractFtile, Ftile, FtileGeometry, Swimable};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::UTranslate;
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::skin::SkinParam;

const HEIGHT: f64 = 1.5;

pub(crate) struct FtileThinSplit {
    base: AbstractFtile,
    width: f64,
    first: f64,
    last: f64,
    /// `None` draws an invisible line, as when all arrows into the split are hidden.
    color_bar: Option<HColor>,
    swimlane: Option<SwimlaneId>,
}

impl FtileThinSplit {
    /// PlantUML places the line (`setGeom`) once the flows it spans are built; here the builder knows
    /// them before it builds the line.
    pub(crate) fn new(
        skin_param: Rc<SkinParam>,
        color_bar: Option<HColor>,
        swimlane: Option<SwimlaneId>,
        first: f64,
        last: f64,
        width: f64,
    ) -> Self {
        Self {
            base: AbstractFtile::new(skin_param),
            width,
            first,
            last,
            color_bar,
            swimlane,
        }
    }
}

impl Swimable for FtileThinSplit {
    fn get_swimlanes(&self) -> SwimlaneSet {
        SwimlaneSet::from([self.swimlane])
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.swimlane
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.swimlane
    }
}

impl Ftile for FtileThinSplit {
    fn skin_param(&self) -> &Rc<SkinParam> {
        self.base.skin_param()
    }

    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base.calculate_dimension(|| {
            FtileGeometry::with_out(self.width, HEIGHT, self.width / 2.0, 0.0, HEIGHT)
        })
    }

    fn draw_u(&self, ug: &UGraphic) {
        let rect = UShape::Line {
            dx: self.last - self.first,
            dy: 0.0,
        };
        ug.apply(UTranslate::new(self.first, 0.0))
            .apply(self.color_bar.clone().unwrap_or(HColor::NONE))
            .apply(UStroke::with_thickness(1.5))
            .draw(&rect);
    }
}
