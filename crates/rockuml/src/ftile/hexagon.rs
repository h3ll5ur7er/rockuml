//! The outlines of conditions (PlantUML's `Hexagon`): a small diamond, a hexagon around a label, or a
//! diamond around one.

use std::rc::Rc;

use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::shape::UShape;
use crate::klimt::stencil::Stencil;

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

/// The outline of a hexagon around `tb`, its points reaching [`HEXAGON_HALF_SIZE`] beyond the text half way down
/// (`asStencil`).
pub(crate) fn as_stencil(tb: Rc<dyn TextBlock>) -> impl Stencil {
    HexagonStencil { tb }
}

struct HexagonStencil {
    tb: Rc<dyn TextBlock>,
}

impl HexagonStencil {
    fn get_delta_x(height: f64, y: f64) -> f64 {
        let p = y / height * 2.0;
        if p <= 1.0 {
            HEXAGON_HALF_SIZE * p
        } else {
            HEXAGON_HALF_SIZE * (2.0 - p)
        }
    }
}

impl Stencil for HexagonStencil {
    fn starting_x(&self, string_bounder: &dyn StringBounder, y: f64) -> f64 {
        let dim = self.tb.calculate_dimension(string_bounder);
        -Self::get_delta_x(dim.height, y)
    }

    fn ending_x(&self, string_bounder: &dyn StringBounder, y: f64) -> f64 {
        let dim = self.tb.calculate_dimension(string_bounder);
        dim.width + Self::get_delta_x(dim.height, y)
    }
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
