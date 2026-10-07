//! A path of cubic Bézier curves, as Graphviz routes edges (PlantUML's `DotPath`).

use super::geom::{RectangleArea, UTranslate, XCubicCurve2D, XPoint2D};
use super::shape::{USegment, UShape};

/// Each curve starts where the previous one ends.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct DotPath {
    beziers: Vec<XCubicCurve2D>,
}

impl DotPath {
    #[must_use]
    pub(crate) fn add_curve(&self, curve: XCubicCurve2D) -> Self {
        let mut beziers = self.beziers.clone();
        beziers.push(curve);
        Self { beziers }
    }

    /// A curve on from where the path ends.
    #[must_use]
    pub(crate) fn add_curve_from_end(&self, pt2: XPoint2D, pt3: XPoint2D, pt4: XPoint2D) -> Self {
        let last = self.beziers.last().expect("a path to continue");
        self.add_curve(XCubicCurve2D::new(last.get_p2(), pt2, pt3, pt4))
    }

    pub(crate) fn get_beziers(&self) -> &[XCubicCurve2D] {
        &self.beziers
    }

    pub(crate) fn get_start_point(&self) -> XPoint2D {
        self.beziers[0].get_p1()
    }

    pub(crate) fn get_end_point(&self) -> XPoint2D {
        self.beziers[self.beziers.len() - 1].get_p2()
    }

    /// Moves the start and its control point; a move longer than the first curve drops that curve.
    pub(crate) fn move_start_point(&mut self, mv: UTranslate) {
        let (mut dx, mut dy) = (mv.dx, mv.dy);
        if self.beziers.len() > 1 && (dx * dx + dy * dy).sqrt() >= self.beziers[0].get_length() {
            dx -= self.beziers[1].x1 - self.beziers[0].x1;
            dy -= self.beziers[1].y1 - self.beziers[0].y1;
            self.beziers.remove(0);
        }
        let first = &mut self.beziers[0];
        first.x1 += dx;
        first.y1 += dy;
        first.ctrlx1 += dx;
        first.ctrly1 += dy;
    }

    pub(crate) fn move_end_point(&mut self, mv: UTranslate) {
        let last = self.beziers.last_mut().expect("a path to move");
        last.x2 += mv.dx;
        last.y2 += mv.dy;
        last.ctrlx2 += mv.dx;
        last.ctrly2 += mv.dy;
    }

    /// The direction the path arrives in at its end, in radians.
    pub(crate) fn get_end_angle(&self) -> f64 {
        let last = self.beziers[self.beziers.len() - 1];
        let (mut dx, mut dy) = (last.x2 - last.ctrlx2, last.y2 - last.ctrly2);
        if dx == 0.0 && dy == 0.0 {
            dx = last.x2 - last.x1;
            dy = last.y2 - last.y1;
        }
        libm::atan2(last.y2 + dy - last.y2, last.x2 + dx - last.x2)
    }

    /// The direction the path leaves its start in, in radians.
    pub(crate) fn get_start_angle(&self) -> f64 {
        let first = self.beziers[0];
        let (mut dx, mut dy) = (first.ctrlx1 - first.x1, first.ctrly1 - first.y1);
        if dx == 0.0 && dy == 0.0 {
            dx = first.x2 - first.x1;
            dy = first.y2 - first.y1;
        }
        libm::atan2(first.y1 + dy - first.y1, first.x1 + dx - first.x1)
    }

    /// The path cut where it leaves `tail` and enters `head`, the boxes of the groups at its ends, as if
    /// Graphviz had clipped it at the clusters (`simulateCompound`).
    #[must_use]
    pub(crate) fn simulate_compound(
        &self,
        head: Option<RectangleArea>,
        tail: Option<RectangleArea>,
    ) -> Self {
        let mut me = self.clone();
        if let Some(tail) = tail
            && tail.contains(self.get_start_point())
        {
            let mut idx = 0;
            while idx + 1 < self.beziers.len() && tail.contains(self.beziers[idx].get_p2()) {
                idx += 1;
            }
            if !tail.contains(self.beziers[idx].get_p2()) {
                let mut result = Vec::new();
                let mut current = self.beziers[idx];
                for _ in 0..8 {
                    let (part1, part2) = current.subdivide();
                    if tail.contains(part1.get_p2()) {
                        current = part2;
                    } else {
                        result.insert(0, part2);
                        current = part1;
                    }
                }
                result.extend_from_slice(&self.beziers[idx + 1..]);
                me = Self { beziers: result };
            }
        }
        if let Some(head) = head
            && head.contains(self.get_end_point())
        {
            let mut result = Vec::new();
            for &bezier in &me.beziers {
                if !head.contains(bezier.get_p2()) {
                    result.push(bezier);
                    continue;
                }
                if head.contains(bezier.get_p1()) {
                    return me;
                }
                let mut current = bezier;
                for _ in 0..8 {
                    let (part1, part2) = current.subdivide();
                    if head.contains(part1.get_p2()) {
                        current = part1;
                    } else {
                        result.push(part1);
                        current = part2;
                    }
                }
                return Self { beziers: result };
            }
        }
        me
    }

    /// The path as drawing surfaces take it (`toUPath`).
    pub(crate) fn to_u_path(&self) -> UShape {
        let mut segments = Vec::with_capacity(self.beziers.len() + 1);
        if let Some(first) = self.beziers.first() {
            segments.push(USegment::MoveTo(first.x1, first.y1));
        }
        segments.extend(self.beziers.iter().map(|bez| USegment::CubicTo {
            ctrl1: (bez.ctrlx1, bez.ctrly1),
            ctrl2: (bez.ctrlx2, bez.ctrly2),
            end: (bez.x2, bez.y2),
        }));
        UShape::Path(segments)
    }
}
