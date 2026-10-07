/// Room kept free around a node, left (`x1`), right (`x2`), top (`y1`) and bottom (`y2`) (PlantUML's
/// `Margins`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Margins {
    pub x1: f64,
    pub x2: f64,
    pub y1: f64,
    pub y2: f64,
}

impl Margins {
    pub(crate) const NONE: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    pub(crate) const fn new(x1: f64, x2: f64, y1: f64, y2: f64) -> Self {
        Self { x1, x2, y1, y2 }
    }

    pub(crate) const fn uniform(value: f64) -> Self {
        Self::new(value, value, value, value)
    }

    /// The larger margin on each side.
    #[must_use]
    pub(crate) fn merge(self, other: Self) -> Self {
        Self::new(
            self.x1.max(other.x1),
            self.x2.max(other.x2),
            self.y1.max(other.y1),
            self.y2.max(other.y2),
        )
    }

    pub(crate) fn is_zero(self) -> bool {
        self == Self::NONE
    }

    pub(crate) fn get_total_width(self) -> f64 {
        self.x1 + self.x2
    }

    pub(crate) fn get_total_height(self) -> f64 {
        self.y1 + self.y2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merging_keeps_the_larger_side() {
        let merged = Margins::new(1.0, 5.0, 0.0, 2.0).merge(Margins::uniform(3.0));
        assert_eq!(merged, Margins::new(3.0, 5.0, 3.0, 3.0));
        assert_eq!(merged.get_total_width(), 8.0);
        assert_eq!(merged.get_total_height(), 6.0);
        assert!(Margins::NONE.is_zero());
        assert!(!merged.is_zero());
    }
}
