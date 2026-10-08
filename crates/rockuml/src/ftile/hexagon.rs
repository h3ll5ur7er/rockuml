//! The outlines of conditions (PlantUML's `Hexagon`): a small diamond, a hexagon around a label, or a
//! diamond around one.

use crate::klimt::shape::UShape;

/// Half the size of an empty diamond (`hexagonHalfSize`), which also sets how pointed hexagons are.
pub(crate) const HEXAGON_HALF_SIZE: f64 = 12.0;

/// The empty diamond (`asPolygon(shadowing)`).
pub(crate) fn as_polygon() -> UShape {
    let h = HEXAGON_HALF_SIZE;
    UShape::polygon(vec![
        (h, 0.0),
        (h * 2.0, h),
        (h, h * 2.0),
        (0.0, h),
        (h, 0.0),
    ])
}

/// A hexagon `width` wide and `height` high (`asPolygon(shadowing, width, height)`).
pub(crate) fn as_polygon_sized(width: f64, height: f64) -> UShape {
    let h = HEXAGON_HALF_SIZE;
    UShape::polygon(vec![
        (h, 0.0),
        (width - h, 0.0),
        (width, height / 2.0),
        (width - h, height),
        (h, height),
        (0.0, height / 2.0),
        (h, 0.0),
    ])
}

/// A diamond `width` wide and `height` high (`asPolygonSquare`).
pub(crate) fn as_polygon_square(width: f64, height: f64) -> UShape {
    UShape::polygon(vec![
        (width / 2.0, 0.0),
        (width, height / 2.0),
        (width / 2.0, height),
        (0.0, height / 2.0),
    ])
}
