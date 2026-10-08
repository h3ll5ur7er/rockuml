//! The small figures that stand for robustness-diagram participants and interfaces (PlantUML's
//! `svek.Boundary`, `svek.Control`, `svek.EntityDomain` and `svek.CircleInterface2`).

use crate::color::HColor;
use crate::klimt::TextBlock;
use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{UEllipse, USegment, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};

const MARGIN: f64 = 4.0;
const RADIUS: f64 = 12.0;

fn circle() -> UShape {
    UShape::Ellipse(UEllipse::new(RADIUS * 2.0, RADIUS * 2.0))
}

/// A circle with a vertical bar on its left.
pub(crate) struct Boundary {
    fashion: Fashion,
}

impl Boundary {
    const LEFT: f64 = 17.0;

    pub(crate) fn new(fashion: Fashion) -> Self {
        Self { fashion }
    }
}

impl TextBlock for Boundary {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(
            RADIUS * 2.0 + Self::LEFT + 2.0 * MARGIN,
            RADIUS * 2.0 + 2.0 * MARGIN,
        )
    }

    fn draw_u(&self, ug: &UGraphic) {
        let ug = self.fashion.apply(ug);
        let bar = vec![
            USegment::MoveTo(0.0, 0.0),
            USegment::LineTo(0.0, RADIUS * 2.0),
            USegment::MoveTo(0.0, RADIUS),
            USegment::LineTo(Self::LEFT, RADIUS),
        ];
        ug.translated(MARGIN, MARGIN)
            .with_backcolor(HColor::NONE)
            .draw(&UShape::path(bar));
        ug.translated(MARGIN + Self::LEFT, MARGIN).draw(&circle());
    }
}

/// A circle with an arrowhead on top.
pub(crate) struct Control {
    fashion: Fashion,
}

impl Control {
    pub(crate) fn new(fashion: Fashion) -> Self {
        Self { fashion }
    }
}

impl TextBlock for Control {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(RADIUS * 2.0 + 2.0 * MARGIN, RADIUS * 2.0 + 2.0 * MARGIN)
    }

    fn draw_u(&self, ug: &UGraphic) {
        const X_WING: f64 = 6.0;
        const Y_APERTURE: f64 = 5.0;
        const X_CONTACT: f64 = 4.0;
        let ug = self.fashion.apply(ug);
        ug.translated(MARGIN, MARGIN).draw(&circle());
        let arrowhead = vec![
            (0.0, 0.0),
            (X_WING, -Y_APERTURE),
            (X_CONTACT, 0.0),
            (X_WING, Y_APERTURE),
            (0.0, 0.0),
        ];
        ug.with_stroke(UStroke::SIMPLE)
            .with_backcolor(self.fashion.fore_color.clone())
            .translated(MARGIN + RADIUS - X_CONTACT, MARGIN)
            .draw(&UShape::polygon(arrowhead));
    }
}

/// A circle standing on a line.
pub(crate) struct EntityDomain {
    fashion: Fashion,
}

impl EntityDomain {
    const SUPP_Y: f64 = 2.0;

    pub(crate) fn new(fashion: Fashion) -> Self {
        Self { fashion }
    }
}

impl TextBlock for EntityDomain {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(RADIUS * 2.0 + 2.0 * MARGIN, RADIUS * 2.0 + 2.0 * MARGIN)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let ug = self.fashion.apply(ug);
        ug.translated(MARGIN, MARGIN).draw(&circle());
        ug.translated(MARGIN, MARGIN + 2.0 * RADIUS + Self::SUPP_Y)
            .draw(&UShape::Line {
                dx: 2.0 * RADIUS,
                dy: 0.0,
            });
    }
}

/// A small circle, in the stroke of the surface it is drawn on.
pub(crate) struct CircleInterface2 {
    background_color: HColor,
    foreground_color: HColor,
}

impl CircleInterface2 {
    const MARGIN: f64 = 1.0;
    const RADIUS: f64 = 8.0;

    pub(crate) fn new(background_color: HColor, foreground_color: HColor) -> Self {
        Self {
            background_color,
            foreground_color,
        }
    }
}

impl TextBlock for CircleInterface2 {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(
            Self::RADIUS * 2.0 + 2.0 * Self::MARGIN,
            Self::RADIUS * 2.0 + 2.0 * Self::MARGIN,
        )
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.with_backcolor(self.background_color.clone())
            .with_color(self.foreground_color.clone())
            .translated(Self::MARGIN, Self::MARGIN)
            .draw(&UShape::Ellipse(UEllipse::new(
                Self::RADIUS * 2.0,
                Self::RADIUS * 2.0,
            )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klimt::debug::StringBounderDebug;

    fn fashion() -> Fashion {
        Fashion::new(HColor::WHITE, HColor::BLACK)
    }

    fn dimension(symbol: &dyn TextBlock) -> XDimension2D {
        symbol.calculate_dimension(&StringBounderDebug)
    }

    #[test]
    fn robustness_symbols_have_fixed_sizes() {
        assert_eq!(
            dimension(&Boundary::new(fashion())),
            XDimension2D::new(49.0, 32.0)
        );
        assert_eq!(
            dimension(&Control::new(fashion())),
            XDimension2D::new(32.0, 32.0)
        );
        assert_eq!(
            dimension(&EntityDomain::new(fashion())),
            XDimension2D::new(32.0, 32.0)
        );
    }
}
