//! The small symbols that stand for robustness-diagram participants and data stores (PlantUML's
//! `svek.Boundary`, `svek.Control`, `svek.EntityDomain`, and `asSmall` of `USymbolDatabase` and
//! `USymbolQueue`).

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

fn cubic(ctrl1: (f64, f64), ctrl2: (f64, f64), end: (f64, f64)) -> USegment {
    USegment::CubicTo { ctrl1, ctrl2, end }
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
            .draw(&UShape::Path(bar));
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
            .draw(&UShape::Polygon(arrowhead));
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

/// The room a symbol leaves around its label (`USymbol.Margin`).
struct Margin {
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Margin {
    fn add_dimension(&self, dimension: XDimension2D) -> XDimension2D {
        dimension.delta(self.x1 + self.x2, self.y1 + self.y2)
    }
}

/// A cylinder standing upright around its label (`USymbolDatabase.asSmall`).
pub(crate) struct SmallDatabase {
    label: Box<dyn TextBlock>,
    fashion: Fashion,
}

impl SmallDatabase {
    const MARGIN: Margin = Margin {
        x1: 10.0,
        x2: 10.0,
        y1: 24.0,
        y2: 5.0,
    };

    pub(crate) fn new(label: Box<dyn TextBlock>, fashion: Fashion) -> Self {
        Self { label, fashion }
    }
}

impl TextBlock for SmallDatabase {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        Self::MARGIN.add_dimension(self.label.calculate_dimension(string_bounder))
    }

    /// The empty square past the bottom right corner makes room for the cylinder's curves.
    fn draw_u(&self, ug: &UGraphic) {
        let XDimension2D { width, height } = self.calculate_dimension(ug.string_bounder());
        let ug = self.fashion.apply(ug);
        ug.draw(&UShape::Path(vec![
            USegment::MoveTo(0.0, 10.0),
            cubic((0.0, 0.0), (width / 2.0, 0.0), (width / 2.0, 0.0)),
            cubic((width / 2.0, 0.0), (width, 0.0), (width, 10.0)),
            USegment::LineTo(width, height - 10.0),
            cubic(
                (width, height),
                (width / 2.0, height),
                (width / 2.0, height),
            ),
            cubic((width / 2.0, height), (0.0, height), (0.0, height - 10.0)),
            USegment::LineTo(0.0, 10.0),
        ]));
        ug.with_backcolor(HColor::NONE).draw(&UShape::Path(vec![
            USegment::MoveTo(0.0, 10.0),
            cubic((0.0, 20.0), (width / 2.0, 20.0), (width / 2.0, 20.0)),
            cubic((width / 2.0, 20.0), (width, 20.0), (width, 10.0)),
        ]));
        ug.translated(width, height)
            .draw(&UShape::Empty(XDimension2D::new(10.0, 10.0)));
        self.label
            .draw_u(&ug.translated(Self::MARGIN.x1, Self::MARGIN.y1));
    }
}

/// A cylinder lying on its side around its label (`USymbolQueue.asSmall`).
pub(crate) struct SmallQueue {
    label: Box<dyn TextBlock>,
    fashion: Fashion,
}

impl SmallQueue {
    const DX: f64 = 5.0;
    const MARGIN: Margin = Margin {
        x1: 5.0,
        x2: 15.0,
        y1: 5.0,
        y2: 5.0,
    };

    pub(crate) fn new(label: Box<dyn TextBlock>, fashion: Fashion) -> Self {
        Self { label, fashion }
    }
}

impl TextBlock for SmallQueue {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        Self::MARGIN.add_dimension(self.label.calculate_dimension(string_bounder))
    }

    fn draw_u(&self, ug: &UGraphic) {
        let dx = Self::DX;
        let XDimension2D { width, height } = self.calculate_dimension(ug.string_bounder());
        let ug = self.fashion.apply(ug);
        ug.draw(&UShape::Path(vec![
            USegment::MoveTo(dx, 0.0),
            USegment::LineTo(width - dx, 0.0),
            cubic((width, 0.0), (width, height / 2.0), (width, height / 2.0)),
            cubic((width, height / 2.0), (width, height), (width - dx, height)),
            USegment::LineTo(dx, height),
            cubic((0.0, height), (0.0, height / 2.0), (0.0, height / 2.0)),
            cubic((0.0, height / 2.0), (0.0, 0.0), (dx, 0.0)),
        ]));
        ug.with_backcolor(HColor::NONE).draw(&UShape::Path(vec![
            USegment::MoveTo(width - dx, 0.0),
            cubic(
                (width - dx * 2.0, 0.0),
                (width - dx * 2.0, height / 2.0),
                (width - dx * 2.0, height / 2.0),
            ),
            cubic(
                (width - dx * 2.0, height),
                (width - dx, height),
                (width - dx, height),
            ),
        ]));
        self.label
            .draw_u(&ug.translated(Self::MARGIN.x1, Self::MARGIN.y1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klimt::debug::StringBounderDebug;
    use crate::skin::component::TextBlockEmpty;

    fn fashion() -> Fashion {
        Fashion::new(HColor::WHITE, HColor::BLACK)
    }

    fn dimension(symbol: &dyn TextBlock) -> XDimension2D {
        symbol.calculate_dimension(&StringBounderDebug)
    }

    fn label(width: f64, height: f64) -> Box<dyn TextBlock> {
        Box::new(TextBlockEmpty {
            dimension: XDimension2D::new(width, height),
        })
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

    #[test]
    fn stores_grow_around_their_label() {
        assert_eq!(
            dimension(&SmallDatabase::new(label(16.0, 17.0), fashion())),
            XDimension2D::new(36.0, 46.0)
        );
        assert_eq!(
            dimension(&SmallQueue::new(label(10.0, 4.0), fashion())),
            XDimension2D::new(30.0, 14.0)
        );
    }
}
