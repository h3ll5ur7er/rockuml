#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct XDimension2D {
    pub width: f64,
    pub height: f64,
}

impl XDimension2D {
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub fn delta(self, dx: f64, dy: f64) -> Self {
        Self::new(self.width + dx, self.height + dy)
    }

    /// The space for `self` with `below` stacked under it.
    #[must_use]
    pub fn merge_top_bottom(self, below: Self) -> Self {
        Self::new(self.width.max(below.width), self.height + below.height)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct XPoint2D {
    pub x: f64,
    pub y: f64,
}

impl XPoint2D {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UTranslate {
    pub dx: f64,
    pub dy: f64,
}

impl UTranslate {
    pub const fn new(dx: f64, dy: f64) -> Self {
        Self { dx, dy }
    }

    #[must_use]
    pub fn compose(self, other: UTranslate) -> Self {
        Self::new(self.dx + other.dx, self.dy + other.dy)
    }
}

/// The bounding box of drawn points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MinMax {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl MinMax {
    /// A box anchored at the origin, so it always includes (0, 0).
    pub(crate) const fn from_origin() -> Self {
        Self {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 0.0,
            max_y: 0.0,
        }
    }

    #[must_use]
    pub(crate) fn add_point(self, x: f64, y: f64) -> Self {
        Self {
            min_x: self.min_x.min(x),
            min_y: self.min_y.min(y),
            max_x: self.max_x.max(x),
            max_y: self.max_y.max(y),
        }
    }

    pub(crate) fn dimension(self) -> XDimension2D {
        XDimension2D::new(self.max_x - self.min_x, self.max_y - self.min_y)
    }
}

/// Space around something, in PlantUML's clockwise order.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ClockwiseTopRightBottomLeft {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl ClockwiseTopRightBottomLeft {
    pub(crate) const fn none() -> Self {
        Self::same(0.0)
    }

    pub(crate) const fn top_right_bottom_left(
        top: f64,
        right: f64,
        bottom: f64,
        left: f64,
    ) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub(crate) const fn same(value: f64) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

/// A vector rotated by `angle` radians, as Java's `XAffineTransform.getRotateInstance` turns it.
pub(crate) fn rotate(x: f64, y: f64, angle: f64) -> (f64, f64) {
    if angle == 0.0 {
        return (x, y);
    }
    let (sin, cos) = angle.sin_cos();
    (cos * x + -sin * y, sin * x + cos * y)
}
