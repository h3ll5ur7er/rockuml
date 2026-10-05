//! The atoms that are not text.

use super::sheet_block::SheetBlock1;
use super::{Atom, Sheet};
use crate::klimt::TextBlock;
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::shape::{UEllipse, URectangle, UShape};
use crate::klimt::stencil::UHorizontalLine;
use crate::klimt::ugraphic::{UGraphic, UStroke};

/// The mark in front of a `*` list item: a disc at the first level, a square below.
pub struct Bullet {
    font: FontConfiguration,
    order: usize,
}

impl Bullet {
    pub fn new(font: FontConfiguration, order: usize) -> Self {
        Self { font, order }
    }
}

impl TextBlock for Bullet {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        if self.order == 0 {
            XDimension2D::new(12.0, 5.0)
        } else {
            XDimension2D::new(8.0 + 8.0 * self.order as f64, 3.0)
        }
    }

    fn draw_u(&self, ug: &UGraphic) {
        let color = self.font.color().clone();
        let ug = ug
            .with_color(color.clone())
            .with_backcolor(color)
            .with_stroke(UStroke::with_thickness(0.0));
        if self.order == 0 {
            ug.translated(3.0, 0.0)
                .draw(&UShape::Ellipse(UEllipse::new(5.0, 5.0)));
        } else {
            ug.translated(1.0 + 8.0 * self.order as f64, 0.0)
                .draw(&UShape::Rectangle(URectangle::new(3.5, 3.5)));
        }
    }
}

impl Atom for Bullet {
    fn starting_altitude(&self, _string_bounder: &dyn StringBounder) -> f64 {
        if self.order == 0 { -5.0 } else { -7.0 }
    }
}

/// A separator line (`----`, `====`, `....`), optionally with a title in its middle (`== Title ==`).
pub struct HorizontalLine {
    /// The character the line is drawn with.
    style: char,
    title: Option<SheetBlock1>,
}

impl HorizontalLine {
    pub fn new(style: char, title: Option<Sheet>) -> Self {
        Self {
            style,
            title: title.map(|sheet| SheetBlock1::new(sheet, ClockwiseTopRightBottomLeft::none())),
        }
    }
}

impl TextBlock for HorizontalLine {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.title
            .as_ref()
            .map_or(XDimension2D::new(10.0, 10.0), |title| {
                title.calculate_dimension(string_bounder)
            })
    }

    fn draw_u(&self, ug: &UGraphic) {
        let height = self.calculate_dimension(ug.string_bounder()).height;
        ug.translated(0.0, height / 2.0)
            .draw_horizontal_line(&UHorizontalLine {
                style: self.style,
                title: self.title.as_ref().map(|title| title as &dyn TextBlock),
            });
    }
}

impl Atom for HorizontalLine {
    fn starting_altitude(&self, _string_bounder: &dyn StringBounder) -> f64 {
        0.0
    }
}
