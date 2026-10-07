//! Where an entity sits on its container (PlantUML's `EntityPosition`), and the parts of an entity `hide`
//! and `show` act on (`EntityPortion`).

use crate::klimt::geom::{XDimension2D, XPoint2D};
use crate::klimt::shape::{UEllipse, URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::skin::Rankdir;

/// Entry and exit points, pins and ports sit on their container's border; normal entities inside.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntityPosition {
    Normal,
    EntryPoint,
    ExitPoint,
    InputPin,
    OutputPin,
    ExpansionInput,
    ExpansionOutput,
    Portin,
    Portout,
}

impl EntityPosition {
    pub(crate) const RADIUS: f64 = 6.0;

    pub(crate) fn is_normal(self) -> bool {
        self == Self::Normal
    }

    /// # Panics
    ///
    /// For normal entities, which have no symbol of their own.
    pub(crate) fn draw_symbol(self, ug: &UGraphic, rankdir: Rankdir) {
        const SIDE: f64 = EntityPosition::RADIUS * 2.0;
        match self {
            Self::Normal => panic!("normal entities have no position symbol"),
            Self::EntryPoint | Self::ExitPoint => {
                ug.draw(&UShape::Ellipse(UEllipse::new(SIDE, SIDE)));
                if self == Self::ExitPoint {
                    let center = Self::RADIUS + 0.5;
                    let radius = Self::RADIUS - 0.5;
                    let on_circle = |angle: f64| {
                        XPoint2D::new(center + radius * angle.cos(), center + radius * angle.sin())
                    };
                    let quarter = std::f64::consts::PI / 4.0;
                    let pi = std::f64::consts::PI;
                    draw_line(ug, on_circle(quarter), on_circle(pi + quarter));
                    draw_line(ug, on_circle(-quarter), on_circle(pi - quarter));
                }
            }
            Self::InputPin | Self::OutputPin => {
                ug.draw(&UShape::Rectangle(URectangle::new(SIDE, SIDE)));
            }
            Self::ExpansionInput | Self::ExpansionOutput => {
                if rankdir == Rankdir::TopToBottom {
                    ug.draw(&UShape::Rectangle(URectangle::new(SIDE * 4.0, SIDE)));
                    for i in 1..4 {
                        ug.translated(SIDE * f64::from(i), 0.0)
                            .draw(&UShape::Line { dx: 0.0, dy: SIDE });
                    }
                } else {
                    ug.draw(&UShape::Rectangle(URectangle::new(SIDE, SIDE * 4.0)));
                    for i in 1..4 {
                        ug.translated(0.0, SIDE * f64::from(i))
                            .draw(&UShape::Line { dx: SIDE, dy: 0.0 });
                    }
                }
            }
            Self::Portin | Self::Portout => {}
        }
    }

    /// Expansion nodes are four squares long, along the rank.
    pub(crate) fn get_dimension(self, rankdir: Rankdir) -> XDimension2D {
        const SIDE: f64 = EntityPosition::RADIUS * 2.0;
        match (self, rankdir) {
            (Self::ExpansionInput | Self::ExpansionOutput, Rankdir::TopToBottom) => {
                XDimension2D::new(SIDE * 4.0, SIDE)
            }
            (Self::ExpansionInput | Self::ExpansionOutput, Rankdir::LeftToRight) => {
                XDimension2D::new(SIDE, SIDE * 4.0)
            }
            _ => XDimension2D::new(SIDE, SIDE),
        }
    }

    /// The position a state's stereotype like `<<entryPoint>>` asks for, ignoring case.
    pub(crate) fn from_stereotype(label: &str) -> Self {
        [
            ("<<entrypoint>>", Self::EntryPoint),
            ("<<exitpoint>>", Self::ExitPoint),
            ("<<inputpin>>", Self::InputPin),
            ("<<outputpin>>", Self::OutputPin),
            ("<<expansioninput>>", Self::ExpansionInput),
            ("<<expansionoutput>>", Self::ExpansionOutput),
        ]
        .into_iter()
        .find(|(name, _)| label.eq_ignore_ascii_case(name))
        .map_or(Self::Normal, |(_, position)| position)
    }
}

fn draw_line(ug: &UGraphic, p1: XPoint2D, p2: XPoint2D) {
    ug.translated(p1.x, p1.y).draw(&UShape::Line {
        dx: p2.x - p1.x,
        dy: p2.y - p1.y,
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntityPortion {
    Field,
    Method,
    Member,
    CircledCharacter,
    Stereotype,
}

impl EntityPortion {
    /// The portions this one stands for: members are fields and methods.
    pub(crate) fn as_set(self) -> Vec<Self> {
        match self {
            Self::Member => vec![Self::Field, Self::Method],
            other => vec![other],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereotypes_place_states_on_the_border() {
        assert_eq!(
            EntityPosition::from_stereotype("<<entryPoint>>"),
            EntityPosition::EntryPoint
        );
        assert_eq!(
            EntityPosition::from_stereotype("<<ExpansionOutput>>"),
            EntityPosition::ExpansionOutput
        );
        assert_eq!(
            EntityPosition::from_stereotype("<<choice>>"),
            EntityPosition::Normal
        );
    }

    #[test]
    fn members_are_fields_and_methods() {
        assert_eq!(
            EntityPortion::Member.as_set(),
            [EntityPortion::Field, EntityPortion::Method]
        );
        assert_eq!(
            EntityPortion::Stereotype.as_set(),
            [EntityPortion::Stereotype]
        );
    }
}
