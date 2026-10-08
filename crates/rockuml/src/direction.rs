//! Which way an arrow written in the source points (PlantUML's `Direction` and
//! `StringUtils.getQueueDirection`).

use crate::klimt::geom::XPoint2D;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
    Right,
    Left,
    Down,
    Up,
}

impl Direction {
    #[must_use]
    pub(crate) fn get_inv(self) -> Self {
        match self {
            Self::Right => Self::Left,
            Self::Left => Self::Right,
            Self::Down => Self::Up,
            Self::Up => Self::Down,
        }
    }

    /// The first letter of the direction's Java name: `R`, `L`, `D` or `U` (`getShortCode`).
    pub(crate) fn short_code(self) -> char {
        match self {
            Self::Right => 'R',
            Self::Left => 'L',
            Self::Down => 'D',
            Self::Up => 'U',
        }
    }

    /// Which way a horizontal or vertical segment from `p1` to `p2` points; `None` when it has no length
    /// (`fromVector`, which returns `null` there). Coordinates compare exactly, as in PlantUML.
    ///
    /// PlantUML fails on a slanted segment, which arrows of activity diagrams never have; here it has no
    /// direction either.
    pub(crate) fn from_vector(p1: XPoint2D, p2: XPoint2D) -> Option<Self> {
        let (x1, y1, x2, y2) = (p1.x, p1.y, p2.x, p2.y);
        if x1 == x2 && y1 == y2 {
            return None;
        }
        if x1 == x2 {
            return Some(if y2 > y1 { Self::Down } else { Self::Up });
        }
        if y1 == y2 {
            return Some(if x2 > x1 { Self::Right } else { Self::Left });
        }
        debug_assert!(false, "not a horizontal or vertical segment: {p1:?} {p2:?}");
        None
    }

    /// `Left` when `p1` lies left of `p2`, `Right` when it lies right of it: the side `p2` is crossed
    /// from, as PlantUML names it; `None` above one another, where PlantUML fails (`leftOrRight`).
    pub(crate) fn left_or_right(p1: XPoint2D, p2: XPoint2D) -> Option<Self> {
        if p1.x < p2.x {
            return Some(Self::Left);
        }
        if p1.x > p2.x {
            return Some(Self::Right);
        }
        None
    }

    /// The direction an arrow's body like `-left-`, `-l-` or `--` names; a body without one points down,
    /// and a single dash right.
    pub(crate) fn of_queue(queue: &str) -> Self {
        let queue = queue.to_lowercase();
        let named = [
            ("left", Self::Left),
            ("right", Self::Right),
            ("up", Self::Up),
            ("down", Self::Down),
            ("l", Self::Left),
            ("r", Self::Right),
            ("u", Self::Up),
            ("d", Self::Down),
        ];
        if let Some((_, direction)) = named.iter().find(|(name, _)| queue.contains(name)) {
            return *direction;
        }
        if queue.chars().count() == 1 {
            Self::Right
        } else {
            Self::Down
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queues_name_their_direction() {
        assert_eq!(Direction::of_queue("-left-"), Direction::Left);
        assert_eq!(Direction::of_queue("UP"), Direction::Up);
        assert_eq!(Direction::of_queue("ri"), Direction::Right);
        assert_eq!(Direction::of_queue("do"), Direction::Down);
        assert_eq!(Direction::of_queue("-"), Direction::Right);
        assert_eq!(Direction::of_queue("--"), Direction::Down);
        assert_eq!(Direction::of_queue(".."), Direction::Down);
    }

    #[test]
    fn segments_point_along_their_axis_and_null_vectors_nowhere() {
        let at = XPoint2D::new;
        assert_eq!(
            Direction::from_vector(at(1.0, 1.0), at(1.0, 5.0)),
            Some(Direction::Down)
        );
        assert_eq!(
            Direction::from_vector(at(1.0, 1.0), at(1.0, -5.0)),
            Some(Direction::Up)
        );
        assert_eq!(
            Direction::from_vector(at(1.0, 1.0), at(3.0, 1.0)),
            Some(Direction::Right)
        );
        assert_eq!(
            Direction::from_vector(at(1.0, 1.0), at(0.0, 1.0)),
            Some(Direction::Left)
        );
        assert_eq!(Direction::from_vector(at(1.0, 1.0), at(1.0, 1.0)), None);
        let code: String = [
            Direction::Right,
            Direction::Left,
            Direction::Down,
            Direction::Up,
        ]
        .map(Direction::short_code)
        .iter()
        .collect();
        assert_eq!(code, "RLDU");
    }
}
