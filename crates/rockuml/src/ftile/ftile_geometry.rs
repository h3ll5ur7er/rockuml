//! The size of a tile and where arrows enter and leave it (PlantUML's `FtileGeometry`).

use super::FtileGeometryMerger;
use crate::klimt::geom::{UTranslate, XDimension2D, XPoint2D};

/// PlantUML's `Double.MIN_NORMAL`, which `out_y` holds for a tile no arrow leaves.
const NO_POINT_OUT: f64 = f64::MIN_POSITIVE;

/// A tile's size, the `left` where arrows enter at `in_y` and leave at `out_y`.
///
/// A tile no arrow leaves (a stop, an end) has `out_y` set to PlantUML's sentinel. Arithmetic on `out_y` is
/// PlantUML's, sentinel or not: [`Self::add_dim`] moves it, which gives such a tile a point out, as in
/// PlantUML.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FtileGeometry {
    width: f64,
    height: f64,
    left: f64,
    in_y: f64,
    out_y: f64,
}

impl FtileGeometry {
    /// A tile no arrow leaves.
    pub(crate) fn new(width: f64, height: f64, left: f64, in_y: f64) -> Self {
        Self::with_out(width, height, left, in_y, NO_POINT_OUT)
    }

    pub(crate) fn with_out(width: f64, height: f64, left: f64, in_y: f64, out_y: f64) -> Self {
        Self {
            width,
            height,
            left,
            in_y,
            out_y,
        }
    }

    pub(crate) fn from_dim(dim: XDimension2D, left: f64, in_y: f64) -> Self {
        Self::new(dim.width, dim.height, left, in_y)
    }

    pub(crate) fn from_dim_with_out(dim: XDimension2D, left: f64, in_y: f64, out_y: f64) -> Self {
        Self::with_out(dim.width, dim.height, left, in_y, out_y)
    }

    pub(crate) fn get_width(&self) -> f64 {
        self.width
    }

    pub(crate) fn get_height(&self) -> f64 {
        self.height
    }

    /// The size alone, as PlantUML uses the geometry as an `XDimension2D`.
    pub(crate) fn dimension(&self) -> XDimension2D {
        XDimension2D::new(self.width, self.height)
    }

    pub(crate) fn get_point_a(&self) -> XPoint2D {
        XPoint2D::new(self.left, self.in_y)
    }

    pub(crate) fn get_point_in(&self) -> XPoint2D {
        XPoint2D::new(self.left, self.in_y)
    }

    /// The middle of the right side. The points B, C, D and out need a point out; PlantUML fails without
    /// one.
    pub(crate) fn get_point_b(&self) -> XPoint2D {
        debug_assert!(self.has_point_out());
        XPoint2D::new(self.width, f64::midpoint(self.in_y, self.out_y))
    }

    pub(crate) fn get_point_c(&self) -> XPoint2D {
        debug_assert!(self.has_point_out());
        XPoint2D::new(self.left, self.out_y)
    }

    pub(crate) fn get_point_d(&self) -> XPoint2D {
        debug_assert!(self.has_point_out());
        XPoint2D::new(0.0, f64::midpoint(self.in_y, self.out_y))
    }

    pub(crate) fn get_point_out(&self) -> XPoint2D {
        debug_assert!(self.has_point_out());
        XPoint2D::new(self.left, self.out_y)
    }

    #[must_use]
    pub(crate) fn inc_height(&self, north_height: f64) -> Self {
        Self {
            height: self.height + north_height,
            ..*self
        }
    }

    /// Room above: everything moves down.
    #[must_use]
    pub(crate) fn add_top(&self, north_height: f64) -> Self {
        let out_y = if self.has_point_out() {
            self.out_y + north_height
        } else {
            NO_POINT_OUT
        };
        Self {
            height: self.height + north_height,
            in_y: self.in_y + north_height,
            out_y,
            ..*self
        }
    }

    #[must_use]
    pub(crate) fn add_bottom(&self, south_height: f64) -> Self {
        Self {
            height: self.height + south_height,
            ..*self
        }
    }

    #[must_use]
    pub(crate) fn inc_right(&self, missing: f64) -> Self {
        Self {
            width: self.width + missing,
            ..*self
        }
    }

    #[must_use]
    pub(crate) fn inc_left(&self, missing: f64) -> Self {
        Self {
            width: self.width + missing,
            left: self.left + missing,
            ..*self
        }
    }

    #[must_use]
    pub(crate) fn inc_vertically(&self, missing1: f64, missing2: f64) -> Self {
        Self {
            height: self.height + missing1 + missing2,
            in_y: self.in_y + missing1,
            out_y: if self.has_point_out() {
                self.out_y + missing1
            } else {
                self.out_y
            },
            ..*self
        }
    }

    #[must_use]
    pub(crate) fn inc_in_y(&self, missing: f64) -> Self {
        Self {
            in_y: self.in_y + missing,
            ..*self
        }
    }

    pub(crate) fn has_point_out(&self) -> bool {
        self.out_y != NO_POINT_OUT
    }

    #[must_use]
    pub(crate) fn without_point_out(&self) -> Self {
        Self::new(self.width, self.height, self.left, self.in_y)
    }

    #[must_use]
    pub(crate) fn translate(&self, translate: UTranslate) -> Self {
        let (dx, dy) = (translate.dx, translate.dy);
        if self.out_y == NO_POINT_OUT {
            return Self::new(self.width, self.height, self.left + dx, self.in_y + dy);
        }
        Self::with_out(
            self.width,
            self.height,
            self.left + dx,
            self.in_y + dy,
            self.out_y + dy,
        )
    }

    pub(crate) fn get_in_y(&self) -> f64 {
        self.in_y
    }

    pub(crate) fn get_left(&self) -> f64 {
        self.left
    }

    pub(crate) fn get_right(&self) -> f64 {
        self.width - self.left
    }

    pub(crate) fn get_out_y(&self) -> f64 {
        self.out_y
    }

    /// Grows the tile; `out_y` moves down by `delta_height` even for a tile no arrow leaves, as in PlantUML.
    #[must_use]
    pub(crate) fn add_dim(&self, delta_width: f64, delta_height: f64) -> Self {
        Self::with_out(
            self.width + delta_width,
            self.height + delta_height,
            self.left,
            self.in_y,
            self.out_y + delta_height,
        )
    }

    #[must_use]
    pub(crate) fn fixed_height(&self, fixed_height: f64) -> Self {
        Self {
            height: fixed_height,
            ..*self
        }
    }

    /// This tile with `other` below it.
    #[must_use]
    pub(crate) fn append_bottom(&self, other: Self) -> Self {
        FtileGeometryMerger::new(*self, other).get_result()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_without_a_point_out_keep_it_through_most_changes() {
        let stop = FtileGeometry::new(20.0, 20.0, 10.0, 0.0);
        assert!(!stop.has_point_out());
        for changed in [
            stop.add_top(5.0),
            stop.add_bottom(5.0),
            stop.inc_vertically(3.0, 4.0),
            stop.inc_height(1.0),
            stop.translate(UTranslate::new(2.0, 3.0)),
        ] {
            assert!(!changed.has_point_out(), "{changed:?}");
        }
        assert_eq!(stop.add_top(5.0).get_in_y(), 5.0);
    }

    #[test]
    fn add_dim_gives_a_tile_without_point_out_one_as_plantuml_does() {
        let stop = FtileGeometry::new(20.0, 20.0, 10.0, 0.0);
        let grown = stop.add_dim(4.0, 6.0);
        assert!(grown.has_point_out());
        assert_eq!(grown.get_out_y(), 6.0);
        assert_eq!(grown.dimension(), XDimension2D::new(24.0, 26.0));
        // Nothing added keeps the sentinel.
        assert!(!stop.add_dim(4.0, 0.0).has_point_out());
    }

    #[test]
    fn points_follow_left_and_the_in_and_out_heights() {
        let box_ = FtileGeometry::with_out(40.0, 30.0, 12.0, 0.0, 30.0);
        assert_eq!(box_.get_point_in(), XPoint2D::new(12.0, 0.0));
        assert_eq!(box_.get_point_out(), XPoint2D::new(12.0, 30.0));
        assert_eq!(box_.get_point_b(), XPoint2D::new(40.0, 15.0));
        assert_eq!(box_.get_point_d(), XPoint2D::new(0.0, 15.0));
        assert_eq!(box_.get_right(), 28.0);
        let moved = box_.translate(UTranslate::new(3.0, 4.0));
        assert_eq!(moved.get_point_out(), XPoint2D::new(15.0, 34.0));
    }
}
