use super::affine::XAffineTransform;
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

    /// At least `min_width` wide and `min_height` high.
    #[must_use]
    pub fn at_least(self, min_width: f64, min_height: f64) -> Self {
        Self::new(self.width.max(min_width), self.height.max(min_height))
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

    #[must_use]
    pub(crate) fn transform(self, transform: &XAffineTransform) -> Self {
        let (x, y) = transform.transform((self.x, self.y));
        Self::new(x, y)
    }
}

/// A rectangle by its top left corner and size.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct XRectangle2D {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl XRectangle2D {
    /// The rectangle moved by `dx` and `dy` (`UTranslate.apply`).
    #[must_use]
    pub(crate) fn translated(self, dx: f64, dy: f64) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            ..self
        }
    }

    pub(crate) fn get_min_x(self) -> f64 {
        self.x
    }

    pub(crate) fn get_max_x(self) -> f64 {
        self.x + self.width
    }

    pub(crate) fn get_center_y(self) -> f64 {
        self.y + self.height / 2.0
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

    pub(crate) fn get_translated(self, point: XPoint2D) -> XPoint2D {
        XPoint2D::new(point.x + self.dx, point.y + self.dy)
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

    /// A box holding no point yet, which the first point added replaces (`MinMax.getEmpty(false)`).
    pub(crate) const fn empty() -> Self {
        Self {
            min_x: f64::MAX,
            min_y: f64::MAX,
            max_x: -f64::MAX,
            max_y: -f64::MAX,
        }
    }

    pub(crate) fn min_x(self) -> f64 {
        self.min_x
    }

    pub(crate) fn min_y(self) -> f64 {
        self.min_y
    }

    pub(crate) fn max_x(self) -> f64 {
        self.max_x
    }

    pub(crate) fn max_y(self) -> f64 {
        self.max_y
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

    #[must_use]
    pub(crate) fn inc_top(self, delta: f64) -> Self {
        Self {
            top: self.top + delta,
            ..self
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

impl XPoint2D {
    #[must_use]
    pub(crate) fn move_by(self, dx: f64, dy: f64) -> Self {
        Self::new(self.x + dx, self.y + dy)
    }

    pub(crate) fn distance(self, other: Self) -> f64 {
        let (dx, dy) = (self.x - other.x, self.y - other.y);
        (dx * dx + dy * dy).sqrt()
    }
}

impl UTranslate {
    pub(crate) fn point(p: XPoint2D) -> Self {
        Self::new(p.x, p.y)
    }

    /// The vector turned by `angle` radians.
    #[must_use]
    pub(crate) fn rotate(self, angle: f64) -> Self {
        Self::point(
            XPoint2D::new(self.dx, self.dy).transform(&XAffineTransform::rotate_instance(angle)),
        )
    }
}

/// An axis-aligned rectangle by its corners (`RectangleArea`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RectangleArea {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl RectangleArea {
    pub(crate) const fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    /// The rectangle with these two opposite corners.
    pub(crate) fn build(pt1: XPoint2D, pt2: XPoint2D) -> Self {
        Self::new(
            pt1.x.min(pt2.x),
            pt1.y.min(pt2.y),
            pt1.x.max(pt2.x),
            pt1.y.max(pt2.y),
        )
    }

    pub(crate) fn get_min_x(self) -> f64 {
        self.min_x
    }

    pub(crate) fn get_min_y(self) -> f64 {
        self.min_y
    }

    pub(crate) fn get_max_x(self) -> f64 {
        self.max_x
    }

    pub(crate) fn get_max_y(self) -> f64 {
        self.max_y
    }

    pub(crate) fn get_width(self) -> f64 {
        self.max_x - self.min_x
    }

    pub(crate) fn get_height(self) -> f64 {
        self.max_y - self.min_y
    }

    pub(crate) fn get_point_center(self) -> XPoint2D {
        XPoint2D::new(
            f64::midpoint(self.min_x, self.max_x),
            f64::midpoint(self.min_y, self.max_y),
        )
    }

    pub(crate) fn get_dimension(self) -> XDimension2D {
        XDimension2D::new(self.get_width(), self.get_height())
    }

    pub(crate) fn get_position(self) -> UTranslate {
        UTranslate::new(self.min_x, self.min_y)
    }

    /// Inside, the right and lower borders excluded.
    pub(crate) fn contains(self, p: XPoint2D) -> bool {
        p.x >= self.min_x && p.x < self.max_x && p.y >= self.min_y && p.y < self.max_y
    }
}

/// A cubic Bézier curve from `p1` to `p2` (`XCubicCurve2D`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct XCubicCurve2D {
    pub x1: f64,
    pub y1: f64,
    pub ctrlx1: f64,
    pub ctrly1: f64,
    pub ctrlx2: f64,
    pub ctrly2: f64,
    pub x2: f64,
    pub y2: f64,
}

impl XCubicCurve2D {
    pub(crate) fn new(p1: XPoint2D, ctrl1: XPoint2D, ctrl2: XPoint2D, p2: XPoint2D) -> Self {
        Self {
            x1: p1.x,
            y1: p1.y,
            ctrlx1: ctrl1.x,
            ctrly1: ctrl1.y,
            ctrlx2: ctrl2.x,
            ctrly2: ctrl2.y,
            x2: p2.x,
            y2: p2.y,
        }
    }

    pub(crate) fn get_p1(self) -> XPoint2D {
        XPoint2D::new(self.x1, self.y1)
    }

    pub(crate) fn get_p2(self) -> XPoint2D {
        XPoint2D::new(self.x2, self.y2)
    }

    pub(crate) fn get_ctrl_p1(self) -> XPoint2D {
        XPoint2D::new(self.ctrlx1, self.ctrly1)
    }

    pub(crate) fn get_ctrl_p2(self) -> XPoint2D {
        XPoint2D::new(self.ctrlx2, self.ctrly2)
    }

    /// The straight distance between the ends.
    pub(crate) fn get_length(self) -> f64 {
        self.get_p1().distance(self.get_p2())
    }

    /// The two halves of the curve, split at its middle parameter.
    pub(crate) fn subdivide(self) -> (Self, Self) {
        let mid = |a: XPoint2D, b: XPoint2D| {
            XPoint2D::new(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y))
        };
        let center = mid(self.get_ctrl_p1(), self.get_ctrl_p2());
        let left_first = mid(self.get_p1(), self.get_ctrl_p1());
        let right_last = mid(self.get_p2(), self.get_ctrl_p2());
        let left_second = mid(left_first, center);
        let right_first = mid(right_last, center);
        let center = mid(left_second, right_first);
        (
            Self::new(self.get_p1(), left_first, left_second, center),
            Self::new(center, right_first, right_last, self.get_p2()),
        )
    }
}
