//! Two tiles one above the other, their entry points aligned (PlantUML's `FtileGeometryMerger`).

use super::FtileGeometry;

pub(crate) struct FtileGeometryMerger {
    result: FtileGeometry,
}

impl FtileGeometryMerger {
    pub(crate) fn new(geo1: FtileGeometry, geo2: FtileGeometry) -> Self {
        let left = geo1.get_left().max(geo2.get_left());
        let dx1 = left - geo1.get_left();
        let dx2 = left - geo2.get_left();
        let width = (geo1.get_width() + dx1).max(geo2.get_width() + dx2);
        let height = geo1.get_height() + geo2.get_height();
        let result = if geo2.has_point_out() {
            FtileGeometry::with_out(
                width,
                height,
                left,
                geo1.get_in_y(),
                geo2.get_out_y() + geo1.get_height(),
            )
        } else {
            FtileGeometry::new(width, height, left, geo1.get_in_y())
        };
        Self { result }
    }

    pub(crate) fn get_result(&self) -> FtileGeometry {
        self.result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lower_tile_moves_under_the_upper_entry_and_gives_the_point_out() {
        let upper = FtileGeometry::with_out(40.0, 30.0, 20.0, 0.0, 30.0);
        let lower = FtileGeometry::with_out(60.0, 10.0, 10.0, 0.0, 10.0);
        let merged = upper.append_bottom(lower);
        assert_eq!(merged.get_left(), 20.0);
        assert_eq!(merged.get_width(), 70.0);
        assert_eq!(merged.get_height(), 40.0);
        assert_eq!(merged.get_out_y(), 40.0);
        let stop = FtileGeometry::new(20.0, 20.0, 10.0, 0.0);
        assert!(!upper.append_bottom(stop).has_point_out());
    }
}
