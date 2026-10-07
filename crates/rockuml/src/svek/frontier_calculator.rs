//! Where the border of a composite state goes so that its entry and exit points sit on it (PlantUML's
//! `FrontierCalculator`).

use crate::abel::EntityPosition;
use crate::klimt::geom::{RectangleArea, XPoint2D};
use crate::skin::Rankdir;

/// How far a border moves out to keep clear of a point near its corner.
const DELTA: f64 = 3.0 * EntityPosition::RADIUS;

pub(crate) struct FrontierCalculator {
    core: RectangleArea,
    initial: RectangleArea,
}

impl FrontierCalculator {
    /// The border around `insides` that the `points` sit on, else where the layout put it, `initial`.
    pub(crate) fn new(
        initial: RectangleArea,
        insides: &[RectangleArea],
        points: &[XPoint2D],
        rankdir: Rankdir,
    ) -> Self {
        let mut core = insides
            .iter()
            .copied()
            .reduce(RectangleArea::merge)
            .unwrap_or_else(|| {
                let center = initial.get_point_center();
                RectangleArea::new(
                    center.x - 1.0,
                    center.y - 1.0,
                    center.x + 1.0,
                    center.y + 1.0,
                )
            });
        for p in points {
            core = core.merge_point(*p);
        }
        if !points.iter().any(|p| p.x == core.get_min_x()) {
            core = core.with_min_x(initial.get_min_x());
        }
        if !points.iter().any(|p| p.x == core.get_max_x()) {
            core = core.with_max_x(initial.get_max_x());
        }
        if !points.iter().any(|p| p.y == core.get_min_y()) {
            core = core.with_min_y(initial.get_min_y());
        }
        if !points.iter().any(|p| p.y == core.get_max_y()) {
            core = core.with_max_y(initial.get_max_y());
        }
        let on_horizontal_side = |p: XPoint2D| p.y == core.get_min_y() || p.y == core.get_max_y();
        let on_vertical_side = |p: XPoint2D| p.x == core.get_min_x() || p.x == core.get_max_x();
        let (mut push_min_x, mut push_max_x, mut push_min_y, mut push_max_y) =
            (false, false, false, false);
        for p in points {
            if on_horizontal_side(*p) {
                push_max_x |= (p.x - core.get_max_x()).abs() < DELTA;
                push_min_x |= (p.x - core.get_min_x()).abs() < DELTA;
            }
            if on_vertical_side(*p) {
                push_max_y |= (p.y - core.get_max_y()).abs() < DELTA;
                push_min_y |= (p.y - core.get_min_y()).abs() < DELTA;
            }
        }
        // A point in a corner stays on the side the graph flows across.
        for p in points {
            if rankdir == Rankdir::LeftToRight {
                if p.x == core.get_min_x() && on_horizontal_side(*p) {
                    push_min_x = false;
                }
                if p.x == core.get_max_x() && on_horizontal_side(*p) {
                    push_max_x = false;
                }
            } else {
                if p.y == core.get_min_y() && on_vertical_side(*p) {
                    push_min_y = false;
                }
                if p.y == core.get_max_y() && on_vertical_side(*p) {
                    push_max_y = false;
                }
            }
        }
        if push_max_x {
            core = core.with_max_x(core.get_max_x() + DELTA);
        }
        if push_min_x {
            core = core.with_min_x(core.get_min_x() - DELTA);
        }
        if push_max_y {
            core = core.with_max_y(core.get_max_y() + DELTA);
        }
        if push_min_y {
            core = core.with_min_y(core.get_min_y() - DELTA);
        }
        Self { core, initial }
    }

    pub(crate) fn get_suggested_position(&self) -> RectangleArea {
        self.core
    }

    /// Widens the border, about its centre, to `min_width`, but not left of where the layout put it.
    pub(crate) fn ensure_min_width(&mut self, min_width: f64) {
        let delta = self.core.get_width() - min_width;
        if delta < 0.0 {
            let mut new_min_x = self.core.get_min_x() + delta / 2.0;
            let mut new_max_x = self.core.get_max_x() - delta / 2.0;
            let error = new_min_x - self.initial.get_min_x();
            if error < 0.0 {
                new_min_x -= error;
                new_max_x -= error;
            }
            self.core = self.core.with_min_x(new_min_x).with_max_x(new_max_x);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_border_reaches_points_outside_and_keeps_clear_of_those_near_corners() {
        let initial = RectangleArea::new(0.0, 0.0, 200.0, 100.0);
        let inside = RectangleArea::new(50.0, 30.0, 150.0, 70.0);
        let entry = XPoint2D::new(140.0, 10.0);
        let frontier = FrontierCalculator::new(initial, &[inside], &[entry], Rankdir::TopToBottom);
        // The top side moves up to the point; the point is far from the corners, so the others stay where the
        // layout put them.
        assert_eq!(
            frontier.get_suggested_position(),
            RectangleArea::new(0.0, 10.0, 200.0, 100.0)
        );
        let near_corner = XPoint2D::new(195.0, 10.0);
        // In a corner, left to right: the right side stays on the point, the top one moves out.
        let frontier =
            FrontierCalculator::new(initial, &[inside], &[near_corner], Rankdir::LeftToRight);
        assert_eq!(
            frontier.get_suggested_position(),
            RectangleArea::new(0.0, 10.0 - DELTA, 195.0, 100.0)
        );
    }

    #[test]
    fn a_title_widens_the_border_about_its_centre() {
        let initial = RectangleArea::new(10.0, 0.0, 30.0, 50.0);
        let mut frontier = FrontierCalculator::new(initial, &[], &[], Rankdir::TopToBottom);
        frontier.ensure_min_width(40.0);
        assert_eq!(
            frontier.get_suggested_position(),
            RectangleArea::new(10.0, 0.0, 50.0, 50.0)
        );
    }
}
