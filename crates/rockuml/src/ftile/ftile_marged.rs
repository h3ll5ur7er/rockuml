//! A tile with room beside or above and below it (PlantUML's `FtileMarged`, `FtileMargedRight` and
//! `FtileMargedVertically`).

use std::cell::OnceCell;
use std::rc::Rc;

use super::vertical::FtileDecorate;
use super::{Ftile, FtileGeometry, Swimable, same};
use crate::diagram::activity3::{LinkRendering, SwimlaneId, SwimlaneSet};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::UTranslate;
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;

/// What a margin does not change is the tile's.
macro_rules! marged_swimable {
    ($tile:ty) => {
        impl Swimable for $tile {
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
    };
}

/// `margin1` left of the tile and `margin2` right of it.
pub(crate) struct FtileMarged {
    tile: Rc<dyn Ftile>,
    margin1: f64,
    margin2: f64,
    cached_geometry: OnceCell<FtileGeometry>,
}

impl FtileMarged {
    pub(crate) fn new(tile: Rc<dyn Ftile>, margin1: f64, margin2: f64) -> Self {
        Self {
            tile,
            margin1,
            margin2,
            cached_geometry: OnceCell::new(),
        }
    }

    fn get_translate(&self) -> UTranslate {
        UTranslate::new(self.margin1, 0.0)
    }
}

marged_swimable!(FtileMarged);

impl Ftile for FtileMarged {
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
            let orig = self.tile.calculate_dimension(string_bounder);
            FtileGeometry::with_out(
                orig.get_width() + self.margin1 + self.margin2,
                orig.get_height(),
                orig.get_left() + self.margin1,
                orig.get_in_y(),
                orig.get_out_y(),
            )
        })
    }

    fn get_translate_for(
        &self,
        child: &dyn Ftile,
        string_bounder: &dyn StringBounder,
    ) -> UTranslate {
        if same(child, self.tile.as_ref()) {
            return self.get_translate();
        }
        self.tile
            .get_translate_for(child, string_bounder)
            .compose(self.get_translate())
    }

    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        vec![self.tile.clone()]
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.apply(self.get_translate()).draw(&self.tile);
    }
}

/// The tile widened to the right, up to `max_x`.
pub(crate) struct FtileMargedRight {
    tile: Rc<dyn Ftile>,
    max_x: f64,
    cached_geometry: OnceCell<FtileGeometry>,
}

impl FtileMargedRight {
    /// `max_x` is wider than the tile.
    pub(crate) fn new(tile: Rc<dyn Ftile>, max_x: f64) -> Self {
        Self {
            tile,
            max_x,
            cached_geometry: OnceCell::new(),
        }
    }
}

marged_swimable!(FtileMargedRight);

impl Ftile for FtileMargedRight {
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
            let orig = self.tile.calculate_dimension(string_bounder);
            FtileGeometry::with_out(
                self.max_x,
                orig.get_height(),
                orig.get_left(),
                orig.get_in_y(),
                orig.get_out_y(),
            )
        })
    }

    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        vec![self.tile.clone()]
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.draw(&self.tile);
    }
}

/// `margin1` above the tile and `margin2` below it. PlantUML draws the tile `margin1` lower, yet says it is
/// drawn where this one is.
pub(crate) struct FtileMargedVertically {
    tile: Rc<dyn Ftile>,
    margin1: f64,
    margin2: f64,
    cached: OnceCell<FtileGeometry>,
}

impl FtileMargedVertically {
    pub(crate) fn new(tile: Rc<dyn Ftile>, margin1: f64, margin2: f64) -> Self {
        Self {
            tile,
            margin1,
            margin2,
            cached: OnceCell::new(),
        }
    }
}

impl FtileDecorate for FtileMargedVertically {
    fn get_ftile_delegated(&self) -> &Rc<dyn Ftile> {
        &self.tile
    }

    fn draw_u(&self, ug: &UGraphic) {
        if self.margin1 > 0.0 {
            ug.apply(UTranslate::new(0.0, self.margin1))
                .draw(&self.tile);
        } else {
            ug.draw(&self.tile);
        }
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        *self.cached.get_or_init(|| {
            self.tile
                .calculate_dimension(string_bounder)
                .inc_vertically(self.margin1, self.margin2)
        })
    }
}
