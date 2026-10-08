//! A tile centred in at least `min_width` (PlantUML's `FtileMinWidthCentered`).

use std::cell::OnceCell;
use std::rc::Rc;

use super::vertical::FtileDecorate;
use super::{Ftile, FtileGeometry, same};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XDimension2D};
use crate::klimt::ugraphic::UGraphic;

pub(crate) struct FtileMinWidthCentered {
    tile: Rc<dyn Ftile>,
    min_width: f64,
    calculate_dimension_internal: OnceCell<FtileGeometry>,
}

impl FtileMinWidthCentered {
    pub(crate) fn new(tile: Rc<dyn Ftile>, min_width: f64) -> Self {
        Self {
            tile,
            min_width,
            calculate_dimension_internal: OnceCell::new(),
        }
    }

    fn calculate_dimension_slow(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        let geo = self.tile.calculate_dimension(string_bounder);
        let left = self.get_point2(geo.get_left(), string_bounder);
        let dim = self.get_dimension_internal(string_bounder);
        if !geo.has_point_out() {
            return FtileGeometry::from_dim(dim, left, geo.get_in_y());
        }
        FtileGeometry::from_dim_with_out(dim, left, geo.get_in_y(), geo.get_out_y())
    }

    fn get_dimension_internal(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let dim = self.tile.calculate_dimension(string_bounder).dimension();
        if dim.width < self.min_width {
            return XDimension2D::new(self.min_width, dim.height);
        }
        dim
    }

    fn get_u_translate_internal(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_tile = self.tile.calculate_dimension(string_bounder);
        let dim_total = self.get_dimension_internal(string_bounder);
        UTranslate::new((dim_total.width - dim_tile.get_width()) / 2.0, 0.0)
    }

    fn get_point2(&self, x: f64, string_bounder: &dyn StringBounder) -> f64 {
        let dim = self.tile.calculate_dimension(string_bounder);
        if dim.get_width() < self.min_width {
            let diff = self.min_width - dim.get_width();
            return x + diff / 2.0;
        }
        x
    }
}

impl FtileDecorate for FtileMinWidthCentered {
    fn get_ftile_delegated(&self) -> &Rc<dyn Ftile> {
        &self.tile
    }

    fn draw_u(&self, ug: &UGraphic) {
        let change = self.get_u_translate_internal(ug.string_bounder());
        self.tile.draw_u(&ug.apply(change));
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        *self
            .calculate_dimension_internal
            .get_or_init(|| self.calculate_dimension_slow(string_bounder))
    }

    /// PlantUML knows only where the wrapped tile is; deeper tiles count as where this one is.
    fn get_translate_for(
        &self,
        child: &dyn Ftile,
        string_bounder: &dyn StringBounder,
    ) -> UTranslate {
        if same(child, self.tile.as_ref()) {
            return self.get_u_translate_internal(string_bounder);
        }
        UTranslate::default()
    }
}
