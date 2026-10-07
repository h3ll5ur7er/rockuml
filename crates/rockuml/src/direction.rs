//! Which way an arrow written in the source points (PlantUML's `Direction` and
//! `StringUtils.getQueueDirection`).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
    Right,
    Left,
    Down,
    Up,
}

impl Direction {
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
}
