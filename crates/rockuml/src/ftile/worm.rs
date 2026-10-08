//! The polyline of an arrow, with the rules that straighten two arrows merged into one (PlantUML's `Worm`).

use std::rc::Rc;

use super::{Arrows, MergeStrategy, Snake};
use crate::decoration::HtmlColorAndStyle;
use crate::direction::Direction;
use crate::klimt::compress::CompressionMode;
use crate::klimt::geom::{UTranslate, XPoint2D};
use crate::klimt::shape::{UPolygon, UShape};
use crate::klimt::ugraphic::{UChange, UGraphic, UStroke};

#[derive(Clone, Debug)]
pub(crate) struct Worm {
    /// Shared with moved copies until one of them changes its points (PlantUML's `sharedPoints`).
    points: Rc<Vec<XPoint2D>>,
    /// What the arrow style draws plain lines with.
    stroke: UStroke,
    arrows: Arrows,
    /// How far this copy is moved. Points are stored unmoved and moved when read, as in PlantUML, so that
    /// coordinates come out of the same additions.
    tr: Option<UTranslate>,
    /// Arrowheads of the arrow take no room when compressing across.
    ignore_for_compression: bool,
}

impl Worm {
    pub(crate) fn new(stroke: UStroke, arrows: Arrows) -> Self {
        Self {
            points: Rc::new(Vec::new()),
            stroke,
            arrows,
            tr: None,
            ignore_for_compression: false,
        }
    }

    /// The same points, `dx` and `dy` further (`move`).
    #[must_use]
    pub(crate) fn move_by(&self, dx: f64, dy: f64) -> Self {
        let step = UTranslate::new(dx, dy);
        Self {
            points: self.points.clone(),
            tr: Some(self.tr.map_or(step, |tr| tr.compose(step))),
            ..self.clone_empty()
        }
    }

    pub(crate) fn is_pure_horizontal(&self) -> bool {
        self.size() == 2 && self.get_point(0).y == self.get_point(1).y
    }

    /// No points, drawn the same way.
    #[must_use]
    pub(crate) fn clone_empty(&self) -> Self {
        Self {
            ignore_for_compression: self.ignore_for_compression,
            ..Self::new(self.stroke, self.arrows)
        }
    }

    /// Only before the first point is added.
    pub(crate) fn set_ignore_for_compression(&mut self) {
        debug_assert_eq!(
            self.size(),
            0,
            "PlantUML refuses this once points are added"
        );
        self.ignore_for_compression = true;
    }

    /// Draws the lines in one colour, the first segment pointing `emphasize_direction` with an arrowhead in
    /// its middle, then the decorations at the ends (`drawInternalOneColor`).
    pub(crate) fn draw_internal_one_color(
        &self,
        start_decoration: Option<&UPolygon>,
        ug: &UGraphic,
        color_and_style: &HtmlColorAndStyle,
        stroke_value: f64,
        emphasize_direction: Option<Direction>,
        end_decoration: Option<&UPolygon>,
    ) {
        let arrow_color = color_and_style.get_arrow_color();
        let link_style = color_and_style.get_style();
        if link_style.is_invisible() {
            return;
        }
        let ug = ug
            .apply(arrow_color.clone())
            .apply(UChange::Background(arrow_color.clone()));
        let ug = if link_style.is_normal() {
            ug.apply(self.stroke)
        } else {
            ug.apply(link_style.go_thickness(stroke_value).get_stroke3())
        };

        let mut drawn = false;
        for i in 0..self.size().saturating_sub(1) {
            let (p1, p2) = (self.get_point(i), self.get_point(i + 1));
            if !drawn
                && emphasize_direction.is_some()
                && Direction::from_vector(p1, p2) == emphasize_direction
            {
                self.draw_line(&ug, p1, p2, emphasize_direction);
                drawn = true;
            } else {
                self.draw_line(&ug, p1, p2, None);
            }
        }

        let arrow_head_color = color_and_style.get_arrow_head_color();
        let ug = ug
            .apply(arrow_head_color.clone())
            .apply(UChange::Background(arrow_head_color.clone()));
        if let Some(decoration) = start_decoration {
            self.draw_decoration(&ug, decoration, self.get_first());
        }
        if let Some(decoration) = end_decoration {
            self.draw_decoration(&ug, decoration, self.get_last());
        }
    }

    fn draw_decoration(&self, ug: &UGraphic, decoration: &UPolygon, at: XPoint2D) {
        // PlantUML marks the arrow's own polygon; arrows build fresh arrowheads each time they are drawn, so
        // marking a copy draws the same.
        let mut decoration = decoration.clone();
        if self.ignore_for_compression {
            decoration.set_compression_mode(CompressionMode::OnX);
        }
        ug.apply(UTranslate::point(at))
            .apply(UStroke::SIMPLE)
            .draw(&UShape::Polygon(decoration));
    }

    fn draw_line(&self, ug: &UGraphic, p1: XPoint2D, p2: XPoint2D, direction: Option<Direction>) {
        let (x1, y1, x2, y2) = (p1.x, p1.y, p2.x, p2.y);
        let ug = ug.apply(UTranslate::new(x1, y1));
        if let Some(direction) = direction {
            ug.apply(UTranslate::new((x2 - x1) / 2.0, (y2 - y1) / 2.0))
                .draw(&UShape::Polygon(self.arrows.as_to(direction)));
        }
        ug.draw(&UShape::Line {
            dx: x2 - x1,
            dy: y2 - y1,
        });
    }

    /// The worm with its first point moved by `move_`, and the second along with it on the moved axis
    /// (`moveFirstPoint`); `move_` is horizontal or vertical.
    #[must_use]
    pub(crate) fn move_first_point(&self, move_: UTranslate) -> Self {
        let (dx, dy) = (move_.dx, move_.dy);
        debug_assert!(dx == 0.0 || dy == 0.0, "PlantUML refuses a slanted move");
        let mut result = Self::new(self.stroke, self.arrows);
        let (mut x0, mut y0) = (self.get_point(0).x, self.get_point(0).y);
        let (mut x1, mut y1) = (self.get_point(1).x, self.get_point(1).y);
        if dx != 0.0 && x0 == x1 {
            x1 += dx;
        }
        if dy != 0.0 && y0 == y1 {
            y1 += dy;
        }
        x0 += dx;
        y0 += dy;
        result.add_point(x0, y0);
        result.add_point(x1, y1);
        for i in 2..self.size() {
            result.add_point_at(self.get_point(i));
        }
        result
    }

    /// The worm with its last point moved by `move_`, and the one before along with it on the moved axis
    /// (`moveLastPoint`); `move_` is horizontal or vertical.
    #[must_use]
    pub(crate) fn move_last_point(&self, move_: UTranslate) -> Self {
        let (dx, dy) = (move_.dx, move_.dy);
        debug_assert!(dx == 0.0 || dy == 0.0, "PlantUML refuses a slanted move");
        let mut result = Self::new(self.stroke, self.arrows);
        let last = self.size() - 1;
        let (mut x8, mut y8) = (self.get_point(last - 1).x, self.get_point(last - 1).y);
        let (mut x9, mut y9) = (self.get_point(last).x, self.get_point(last).y);
        if dx != 0.0 && x8 == x9 {
            x8 += dx;
        }
        if dy != 0.0 && y8 == y9 {
            y8 += dy;
        }
        x9 += dx;
        y9 += dy;
        for i in 0..last - 1 {
            result.add_point_at(self.get_point(i));
        }
        result.add_point(x8, y8);
        result.add_point(x9, y9);
        result
    }

    /// Adds a point where this copy is; a point repeating the last one adds nothing.
    pub(crate) fn add_point(&mut self, x: f64, y: f64) {
        let (x, y) = match self.tr {
            Some(tr) => (x - tr.dx, y - tr.dy),
            None => (x, y),
        };
        debug_assert!(!x.is_nan() && !y.is_nan(), "PlantUML refuses NaN points");
        if self.size() > 0 {
            let last = self.get_last();
            if last.x == x && last.y == y {
                return;
            }
        }
        Rc::make_mut(&mut self.points).push(XPoint2D::new(x, y));
    }

    pub(crate) fn add_point_at(&mut self, point: XPoint2D) {
        self.add_point(point.x, point.y);
    }

    /// The direction of each segment as a letter, such as `DRD` (`getDirectionsCode`).
    pub(crate) fn get_directions_code(&self) -> String {
        (0..self.size().saturating_sub(1))
            .filter_map(|i| Direction::from_vector(self.get_point(i), self.get_point(i + 1)))
            .map(Direction::short_code)
            .collect()
    }

    fn get_pattern_at(&self, i: usize) -> [Option<Direction>; 4] {
        [0, 1, 2, 3].map(|k| self.get_direction_at_point(i + k))
    }

    fn is_forward_and_backward_at(&self, i: usize) -> bool {
        self.get_direction_at_point(i) == self.get_direction_at_point(i + 1).map(Direction::get_inv)
    }

    fn get_direction_at_point(&self, i: usize) -> Option<Direction> {
        Direction::from_vector(self.get_point(i), self.get_point(i + 1))
    }

    pub(crate) fn size(&self) -> usize {
        self.points.len()
    }

    pub(crate) fn get_point(&self, i: usize) -> XPoint2D {
        self.resolve(self.points[i])
    }

    fn resolve(&self, point: XPoint2D) -> XPoint2D {
        match self.tr {
            None => point,
            Some(tr) => tr.get_translated(point),
        }
    }

    pub(crate) fn get_first(&self) -> XPoint2D {
        self.get_point(0)
    }

    pub(crate) fn get_last(&self) -> XPoint2D {
        self.get_point(self.size() - 1)
    }

    pub(crate) fn get_min_x(&self) -> f64 {
        self.points.iter().fold(self.get_point(0).x, |result, &pt| {
            result.min(self.resolve(pt).x)
        })
    }

    pub(crate) fn get_max_x(&self) -> f64 {
        self.points.iter().fold(self.get_point(0).x, |result, &pt| {
            result.max(self.resolve(pt).x)
        })
    }

    pub(crate) fn get_max_y(&self) -> f64 {
        self.points.iter().fold(self.get_point(0).y, |result, &pt| {
            result.max(self.resolve(pt).y)
        })
    }

    /// This worm followed by `other`, which starts where this one ends, straightened by `merge`'s rules.
    /// The result is no longer ignored when compressing, as in PlantUML.
    pub(crate) fn merge(&self, other: &Self, merge: MergeStrategy) -> Self {
        debug_assert!(Snake::same(self.get_last(), other.get_first()));
        let mut result = Self::new(self.stroke, self.arrows);
        for &pt in self.points.iter() {
            result.add_point_at(self.resolve(pt));
        }
        for &pt in other.points.iter() {
            result.add_point_at(other.resolve(pt));
        }
        result.merge_me(merge);
        result
    }

    /// Applies the first rule that changes something until none does; rules are tried in PlantUML's order.
    fn merge_me(&mut self, merge: MergeStrategy) {
        loop {
            let change = self.remove_null_vector()
                || self.remove_redondant_direction()
                || self.remove_pattern1()
                || self.remove_pattern2()
                || self.remove_pattern3()
                || self.remove_pattern4()
                || self.remove_pattern5()
                || self.remove_pattern6()
                || self.remove_pattern7()
                || (merge == MergeStrategy::Full && self.remove_pattern8());
            if !change {
                return;
            }
        }
    }

    fn points_mut(&mut self) -> &mut Vec<XPoint2D> {
        Rc::make_mut(&mut self.points)
    }

    /// Replaces the points `i + 1 ..= i + last` by `new_point`.
    fn replace_by(&mut self, i: usize, last: usize, new_point: XPoint2D) {
        self.points_mut().splice(i + 1..=i + last, [new_point]);
    }

    fn remove_null_vector(&mut self) -> bool {
        for i in 0..self.size().saturating_sub(1) {
            if self.get_direction_at_point(i).is_none() {
                self.points_mut().remove(i);
                return true;
            }
        }
        false
    }

    fn remove_redondant_direction(&mut self) -> bool {
        for i in 0..self.size().saturating_sub(2) {
            if self.get_direction_at_point(i) == self.get_direction_at_point(i + 1) {
                self.points_mut().remove(i + 1);
                return true;
            }
        }
        false
    }

    fn remove_pattern1(&mut self) -> bool {
        use Direction::{Down, Left, Right};
        for i in 0..self.size().saturating_sub(5) {
            let pattern = self.get_pattern_at(i);
            if pattern == [Some(Down), Some(Left), Some(Down), Some(Right)]
                || pattern == [Some(Down), Some(Right), Some(Down), Some(Left)]
            {
                let new_point = XPoint2D::new(self.get_point(i + 1).x, self.get_point(i + 3).y);
                self.replace_by(i, 3, new_point);
                return true;
            }
        }
        false
    }

    fn remove_pattern2(&mut self) -> bool {
        use Direction::{Down, Left, Right, Up};
        for i in 0..self.size().saturating_sub(5) {
            let pattern = self.get_pattern_at(i);
            if pattern == [Some(Right), Some(Down), Some(Right), Some(Up)]
                || pattern == [Some(Left), Some(Down), Some(Left), Some(Up)]
            {
                let new_point = XPoint2D::new(self.get_point(i + 3).x, self.get_point(i + 1).y);
                self.replace_by(i, 3, new_point);
                return true;
            }
        }
        false
    }

    fn remove_pattern3(&mut self) -> bool {
        use Direction::{Down, Left, Right};
        for i in 0..self.size().saturating_sub(4) {
            let pattern = self.get_pattern_at(i);
            if pattern == [Some(Down), Some(Right), Some(Down), Some(Right)]
                || pattern == [Some(Down), Some(Left), Some(Down), Some(Left)]
            {
                let new_point = XPoint2D::new(self.get_point(i + 1).x, self.get_point(i + 3).y);
                self.replace_by(i, 3, new_point);
                return true;
            }
        }
        false
    }

    fn remove_pattern4(&mut self) -> bool {
        use Direction::{Down, Left, Right};
        let Some(i) = self.size().checked_sub(5) else {
            return false;
        };
        if self.get_pattern_at(i) == [Some(Down), Some(Left), Some(Down), Some(Right)]
            && self.get_point(i + 4).x > self.get_point(i + 1).x
        {
            let new_point = XPoint2D::new(self.get_point(i + 1).x, self.get_point(i + 3).y);
            self.replace_by(i, 3, new_point);
            return true;
        }
        false
    }

    fn remove_pattern5(&mut self) -> bool {
        use Direction::{Down, Left, Right};
        let Some(i) = self.size().checked_sub(5) else {
            return false;
        };
        if self.get_pattern_at(i) == [Some(Down), Some(Right), Some(Down), Some(Left)]
            && self.get_point(i + 4).x + 4.0 < self.get_point(i + 1).x
        {
            let new_point = XPoint2D::new(self.get_point(i + 1).x, self.get_point(i + 3).y);
            self.replace_by(i, 3, new_point);
            return true;
        }
        false
    }

    fn remove_pattern6(&mut self) -> bool {
        for i in 0..self.size().saturating_sub(2) {
            if self.is_forward_and_backward_at(i) {
                self.points_mut().remove(i + 1);
                return true;
            }
        }
        false
    }

    /// Only at the start of the worm.
    fn remove_pattern7(&mut self) -> bool {
        use Direction::{Down, Left, Right};
        if self.size() > 4
            && self.get_pattern_at(0) == [Some(Right), Some(Down), Some(Left), Some(Down)]
            && self.get_point(3).x > self.get_point(0).x
        {
            let new_point = XPoint2D::new(self.get_point(3).x, self.get_point(0).y);
            self.replace_by(0, 2, new_point);
            return true;
        }
        false
    }

    fn remove_pattern8(&mut self) -> bool {
        use Direction::{Down, Left, Right};
        for i in 0..self.size().saturating_sub(4) {
            let pattern = self.get_pattern_at(i);
            if pattern == [Some(Left), Some(Down), Some(Left), Some(Down)]
                || pattern == [Some(Right), Some(Down), Some(Right), Some(Down)]
            {
                let new_point = XPoint2D::new(self.get_point(i + 3).x, self.get_point(i + 1).y);
                self.replace_by(i, 3, new_point);
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_an_end_point_forgets_that_arrowheads_take_no_room() {
        let mut worm = Worm::new(UStroke::SIMPLE, Arrows::Regular);
        worm.set_ignore_for_compression();
        worm.add_point(0.0, 0.0);
        worm.add_point(0.0, 10.0);
        let moved = UTranslate::new(5.0, 0.0);
        assert!(!worm.move_first_point(moved).ignore_for_compression);
        assert!(!worm.move_last_point(moved).ignore_for_compression);
        assert!(worm.move_by(5.0, 0.0).ignore_for_compression);
    }
}
