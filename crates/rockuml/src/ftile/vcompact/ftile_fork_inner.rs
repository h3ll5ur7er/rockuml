//! PlantUML's `FtileForkInner`: the flows of a fork or split side by side, which
//! `FtileFactoryDelegatorCreateParallel` then joins with bars and arrows.

use std::cell::OnceCell;
use std::rc::Rc;

use crate::diagram::activity3::{SwimlaneId, SwimlaneSet};
use crate::ftile::{Ftile, FtileGeometry, Swimable, same};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XDimension2D};
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;

pub(crate) struct FtileForkInner {
    /// Never empty: a fork has a first flow.
    forks: Vec<Rc<dyn Ftile>>,
    cached_geometry: OnceCell<FtileGeometry>,
}

impl FtileForkInner {
    pub(crate) fn new(forks: Vec<Rc<dyn Ftile>>) -> Self {
        debug_assert!(!forks.is_empty(), "a fork has a first flow");
        Self {
            forks,
            cached_geometry: OnceCell::new(),
        }
    }

    fn calculate_dimension_ftile(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        let mut height: f64 = 0.0;
        let mut width = 0.0;
        for ftile in &self.forks {
            let dim = ftile.calculate_dimension(string_bounder);
            width += dim.get_width();
            if dim.get_height() > height {
                height = dim.get_height();
            }
        }
        let dim_total = XDimension2D::new(width, height);
        FtileGeometry::from_dim_with_out(dim_total, dim_total.width / 2.0, 0.0, dim_total.height)
    }
}

impl Swimable for FtileForkInner {
    fn get_swimlanes(&self) -> SwimlaneSet {
        self.forks
            .iter()
            .flat_map(|tile| tile.get_swimlanes())
            .collect()
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.forks[0].get_swimlane_in()
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.get_swimlane_in()
    }
}

impl Ftile for FtileForkInner {
    fn skin_param(&self) -> &SkinParam {
        self.forks[0].skin_param()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        *self
            .cached_geometry
            .get_or_init(|| self.calculate_dimension_ftile(string_bounder))
    }

    fn get_translate_for(
        &self,
        searched: &dyn Ftile,
        string_bounder: &dyn StringBounder,
    ) -> UTranslate {
        let mut xpos = 0.0;
        for ftile in &self.forks {
            if same(ftile.as_ref(), searched) {
                return UTranslate::new(xpos, 0.0);
            }
            xpos += ftile.calculate_dimension(string_bounder).get_width();
        }
        UTranslate::default()
    }

    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        self.forks.clone()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let mut xpos = 0.0;
        for ftile in &self.forks {
            ug.apply(UTranslate::new(xpos, 0.0)).draw(ftile);
            xpos += ftile.calculate_dimension(string_bounder).get_width();
        }
    }
}
