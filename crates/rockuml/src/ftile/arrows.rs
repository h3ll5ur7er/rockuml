//! The arrowheads of activity diagrams (PlantUML's `klimt.Arrows`, `ArrowsRegular` and `ArrowsTriangle`):
//! polygons with their tip at the origin.

use crate::direction::Direction;
use crate::klimt::shape::UPolygon;
use crate::skin::SkinParam;

const DELTA1: f64 = 10.0;
const DELTA2: f64 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Arrows {
    /// Notched heads.
    Regular,
    /// Plain triangles, for `skinparam style strictuml`.
    Triangle,
}

impl SkinParam {
    /// The arrowheads activity diagrams draw.
    pub(crate) fn arrows(&self) -> Arrows {
        if self.strict_uml_style() {
            Arrows::Triangle
        } else {
            Arrows::Regular
        }
    }
}

impl Arrows {
    pub(crate) fn as_to_up(self) -> UPolygon {
        UPolygon::new(match self {
            Self::Regular => vec![
                (-DELTA2, DELTA1),
                (0.0, 0.0),
                (DELTA2, DELTA1),
                (0.0, DELTA1 - 4.0),
            ],
            Self::Triangle => vec![(-DELTA2, DELTA1), (0.0, 0.0), (DELTA2, DELTA1)],
        })
    }

    pub(crate) fn as_to_down(self) -> UPolygon {
        UPolygon::new(match self {
            Self::Regular => vec![
                (-DELTA2, -DELTA1),
                (0.0, 0.0),
                (DELTA2, -DELTA1),
                (0.0, -DELTA1 + 4.0),
            ],
            Self::Triangle => vec![(-DELTA2, -DELTA1), (DELTA2, -DELTA1), (0.0, 0.0)],
        })
    }

    pub(crate) fn as_to_right(self) -> UPolygon {
        UPolygon::new(match self {
            Self::Regular => vec![
                (-DELTA1, -DELTA2),
                (0.0, 0.0),
                (-DELTA1, DELTA2),
                (-DELTA1 + 4.0, 0.0),
            ],
            Self::Triangle => vec![(-DELTA1, -DELTA2), (0.0, 0.0), (-DELTA1, DELTA2)],
        })
    }

    pub(crate) fn as_to_left(self) -> UPolygon {
        UPolygon::new(match self {
            Self::Regular => vec![
                (DELTA1, -DELTA2),
                (0.0, 0.0),
                (DELTA1, DELTA2),
                (DELTA1 - 4.0, 0.0),
            ],
            Self::Triangle => vec![(DELTA1, -DELTA2), (0.0, 0.0), (DELTA1, DELTA2)],
        })
    }

    pub(crate) fn as_to(self, direction: Direction) -> UPolygon {
        match direction {
            Direction::Up => self.as_to_up(),
            Direction::Down => self.as_to_down(),
            Direction::Left => self.as_to_left(),
            Direction::Right => self.as_to_right(),
        }
    }
}
