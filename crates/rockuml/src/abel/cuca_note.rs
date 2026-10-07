//! Notes attached to entities and links, and tips attached to members (PlantUML's `CucaNote`,
//! `NoteLinkStrategy`, `Tip` and `utils.Position`).

use crate::color::Colors;
use crate::creole::Display;
use crate::direction::Direction;
use crate::klimt::geom::XDimension2D;
use crate::skin::Rankdir;
use crate::stereo::Stereotype;

/// The side of its target a note goes on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Position {
    Right,
    Left,
    Bottom,
    Top,
}

impl Position {
    /// `right`, `left`, `bottom` or `top`, in any case.
    pub(crate) fn from_string(s: &str) -> Option<Self> {
        [
            ("RIGHT", Self::Right),
            ("LEFT", Self::Left),
            ("BOTTOM", Self::Bottom),
            ("TOP", Self::Top),
        ]
        .into_iter()
        .find(|(name, _)| s.to_uppercase() == *name)
        .map(|(_, position)| position)
    }

    /// The side in a diagram flowing `rankdir`: left to right turns sides a quarter.
    #[must_use]
    pub(crate) fn with_rankdir(self, rankdir: Rankdir) -> Self {
        if rankdir == Rankdir::TopToBottom {
            return self;
        }
        match self {
            Self::Right => Self::Bottom,
            Self::Left => Self::Top,
            Self::Bottom => Self::Right,
            Self::Top => Self::Left,
        }
    }

    /// Where a tip on this side points: back towards its entity.
    ///
    /// # Panics
    ///
    /// For top and bottom, where tips never go.
    pub(crate) fn reverse_direction(self) -> Direction {
        match self {
            Self::Left => Direction::Right,
            Self::Right => Direction::Left,
            Self::Top | Self::Bottom => panic!("tips go left or right"),
        }
    }

    /// `RIGHT`, `LEFT`, `BOTTOM` or `TOP`.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Right => "RIGHT",
            Self::Left => "LEFT",
            Self::Bottom => "BOTTOM",
            Self::Top => "TOP",
        }
    }
}

/// How much of a link note a link prints, when a link cut in two shares one note between its halves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NoteLinkStrategy {
    Normal,
    HalfPrintedFull,
    HalfNotPrinted,
}

impl NoteLinkStrategy {
    pub(crate) fn compute_dimension(self, width: f64, height: f64) -> XDimension2D {
        match self {
            Self::Normal => XDimension2D::new(width, height),
            Self::HalfPrintedFull => XDimension2D::new(width / 2.0, height),
            Self::HalfNotPrinted => XDimension2D::new(0.0, 0.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CucaNote {
    pub display: Display,
    pub position: Position,
    pub colors: Colors,
    pub strategy: NoteLinkStrategy,
}

impl CucaNote {
    pub(crate) fn build(display: Display, position: Position, colors: Colors) -> Self {
        Self {
            display,
            position,
            colors,
            strategy: NoteLinkStrategy::Normal,
        }
    }

    #[must_use]
    pub(crate) fn with_strategy(&self, strategy: NoteLinkStrategy) -> Self {
        Self {
            strategy,
            ..self.clone()
        }
    }
}

/// One `note left of Class::member`. Tips on the same side of a class share one entity, so each keeps its
/// own colours and stereotype.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Tip {
    pub display: Display,
    pub colors: Colors,
    pub stereotype: Option<Stereotype>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halves_of_a_cut_link_share_its_note() {
        assert_eq!(
            NoteLinkStrategy::HalfPrintedFull.compute_dimension(40.0, 10.0),
            XDimension2D::new(20.0, 10.0)
        );
        assert_eq!(
            NoteLinkStrategy::HalfNotPrinted.compute_dimension(40.0, 10.0),
            XDimension2D::new(0.0, 0.0)
        );
        let note = CucaNote::build(Display::create(["n"]), Position::Top, Colors::default());
        assert_eq!(
            note.with_strategy(NoteLinkStrategy::HalfNotPrinted)
                .strategy,
            NoteLinkStrategy::HalfNotPrinted
        );
        assert_eq!(Position::from_string("Left"), Some(Position::Left));
    }
}
