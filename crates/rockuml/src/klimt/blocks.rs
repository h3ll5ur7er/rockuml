//! Text blocks that frame, pad or stack other text blocks.

use super::font::StringBounder;
use super::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use super::shape::{URectangle, UShape};
use super::ugraphic::{UGraphic, UStroke};
use super::{HorizontalAlignment, TextBlock};
use crate::color::HColor;

/// A block inside padding, with a border and background drawn around both.
pub struct Bordered<T> {
    inner: T,
    stroke: UStroke,
    border: HColor,
    background: HColor,
    corner: f64,
    padding: ClockwiseTopRightBottomLeft,
}

impl<T: TextBlock> Bordered<T> {
    pub fn new(
        inner: T,
        stroke: UStroke,
        border: HColor,
        background: HColor,
        corner: f64,
        padding: ClockwiseTopRightBottomLeft,
    ) -> Self {
        Self {
            inner,
            stroke,
            border,
            background,
            corner,
            padding,
        }
    }

    fn frame(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let padding = self.padding;
        self.inner
            .calculate_dimension(string_bounder)
            .delta(padding.left + padding.right, padding.top + padding.bottom)
    }
}

impl<T: TextBlock> TextBlock for Bordered<T> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.frame(string_bounder).delta(1.0, 1.0)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let background =
            if self.background.is_transparent() || &self.background == ug.default_background() {
                HColor::NONE
            } else {
                self.background.clone()
            };
        let border = if self.stroke.thickness == 0.0 {
            background.clone()
        } else {
            self.border.clone()
        };
        if !background.is_transparent() || !border.is_transparent() {
            let frame = self.frame(ug.string_bounder());
            let rectangle = URectangle::new(frame.width, frame.height).rounded(self.corner);
            ug.with_backcolor(background)
                .with_color(border.clone())
                .with_stroke(self.stroke)
                .draw(&UShape::Rectangle(rectangle));
        }
        self.inner.draw_u(
            &ug.with_color(border)
                .translated(self.padding.left, self.padding.top),
        );
    }
}

/// A block with empty space around it.
pub struct Marged<T> {
    inner: T,
    margin: ClockwiseTopRightBottomLeft,
}

impl<T: TextBlock> Marged<T> {
    pub fn new(inner: T, margin: ClockwiseTopRightBottomLeft) -> Self {
        Self { inner, margin }
    }
}

impl<T: TextBlock> TextBlock for Marged<T> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let margin = self.margin;
        self.inner
            .calculate_dimension(string_bounder)
            .delta(margin.left + margin.right, margin.top + margin.bottom)
    }

    /// The whole area is drawn as an empty shape first, which only some formats show.
    fn draw_u(&self, ug: &UGraphic) {
        let dimension = self.calculate_dimension(ug.string_bounder());
        if dimension.width > 0.0 {
            ug.draw(&UShape::Empty(dimension));
            self.inner
                .draw_u(&ug.translated(self.margin.left, self.margin.top));
        }
    }
}

/// A block with other blocks above and below it, as titles, captions, legends, headers and footers are.
pub struct Decorated<'a> {
    original: Box<dyn TextBlock + 'a>,
    top: Option<(Box<dyn TextBlock + 'a>, HorizontalAlignment)>,
    bottom: Option<(Box<dyn TextBlock + 'a>, HorizontalAlignment)>,
}

impl<'a> Decorated<'a> {
    pub fn new(
        original: Box<dyn TextBlock + 'a>,
        top: Option<(Box<dyn TextBlock + 'a>, HorizontalAlignment)>,
        bottom: Option<(Box<dyn TextBlock + 'a>, HorizontalAlignment)>,
    ) -> Self {
        Self {
            original,
            top,
            bottom,
        }
    }
}

fn dimension_of(
    decoration: Option<&(Box<dyn TextBlock + '_>, HorizontalAlignment)>,
    string_bounder: &dyn StringBounder,
) -> XDimension2D {
    decoration.map_or_else(XDimension2D::default, |(block, _)| {
        block.calculate_dimension(string_bounder)
    })
}

impl TextBlock for Decorated<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let top = dimension_of(self.top.as_ref(), string_bounder);
        let bottom = dimension_of(self.bottom.as_ref(), string_bounder);
        let original = self.original.calculate_dimension(string_bounder);
        original.merge_top_bottom(top.merge_top_bottom(bottom))
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let total = self.calculate_dimension(string_bounder);
        let original = self.original.calculate_dimension(string_bounder);
        let top_height = dimension_of(self.top.as_ref(), string_bounder).height;
        let x_of = |block: &dyn TextBlock, alignment: HorizontalAlignment| {
            let width = block.calculate_dimension(string_bounder).width;
            match alignment {
                HorizontalAlignment::Left => 0.0,
                HorizontalAlignment::Center => (total.width - width) / 2.0,
                HorizontalAlignment::Right => total.width - width,
            }
        };
        if let Some((top, alignment)) = &self.top {
            top.draw_u(&ug.translated(x_of(top.as_ref(), *alignment), 0.0));
        }
        self.original
            .draw_u(&ug.translated((total.width - original.width) / 2.0, top_height));
        if let Some((bottom, alignment)) = &self.bottom {
            let x = x_of(bottom.as_ref(), *alignment);
            bottom.draw_u(&ug.translated(x, top_height + original.height));
        }
    }
}
