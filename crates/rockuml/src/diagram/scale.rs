//! How `scale` commands resize the image (PlantUML's `Scale` implementations).

/// A factor, or a size the image is fitted to or capped at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Scale {
    Factor(f64),
    Width(f64),
    Height(f64),
    WidthAndHeight(f64, f64),
    MaxWidth(f64),
    MaxHeight(f64),
    MaxWidthAndHeight(f64, f64),
}

/// PlantUML enlarges at most fourfold; a factor that is not positive leaves the size alone.
const MAX_FACTOR: f64 = 4.0;

impl Scale {
    /// The factor for an image of this size.
    pub(crate) fn factor(self, width: f64, height: f64) -> f64 {
        let capped_at_one = |factor: f64| factor.min(1.0);
        let factor = match self {
            Self::Factor(factor) => factor,
            Self::Width(target) => target / width,
            Self::Height(target) => target / height,
            Self::WidthAndHeight(target_width, target_height) => {
                (target_width / width).min(target_height / height)
            }
            Self::MaxWidth(max) => capped_at_one(max / width),
            Self::MaxHeight(max) => capped_at_one(max / height),
            Self::MaxWidthAndHeight(max_width, max_height) => {
                capped_at_one((max_width / width).min(max_height / height))
            }
        };
        if factor <= 0.0 {
            1.0
        } else {
            factor.min(MAX_FACTOR)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factors_fit_or_cap_the_size() {
        assert_eq!(Scale::Factor(1.5).factor(100.0, 50.0), 1.5);
        assert_eq!(Scale::Width(200.0).factor(100.0, 50.0), 2.0);
        assert_eq!(Scale::WidthAndHeight(200.0, 50.0).factor(100.0, 50.0), 1.0);
        assert_eq!(Scale::MaxWidth(200.0).factor(100.0, 50.0), 1.0);
        assert_eq!(Scale::MaxHeight(25.0).factor(100.0, 50.0), 0.5);
    }

    #[test]
    fn factors_stay_between_zero_and_four() {
        assert_eq!(Scale::Factor(9.0).factor(1.0, 1.0), 4.0);
        assert_eq!(Scale::Width(0.0).factor(1.0, 1.0), 1.0);
    }
}
