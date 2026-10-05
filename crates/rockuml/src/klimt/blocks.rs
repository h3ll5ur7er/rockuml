//! Text blocks that frame, pad or stack other text blocks.

use super::font::{FontConfiguration, StringBounder};
use super::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use super::shape::{URectangle, UShape, UText};
use super::ugraphic::{UGraphic, UStroke};
use super::{HorizontalAlignment, TextBlock};
use crate::color::HColor;
use crate::jaws::BLOCK_E1_REAL_TABULATION;

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

/// Blocks stacked top to bottom, each on the full width in its own background colour if it has one.
pub struct Vertical<'a> {
    blocks: Vec<Box<dyn TextBlock + 'a>>,
    alignment: HorizontalAlignment,
}

impl<'a> Vertical<'a> {
    pub fn new(blocks: Vec<Box<dyn TextBlock + 'a>>, alignment: HorizontalAlignment) -> Self {
        Self { blocks, alignment }
    }
}

impl TextBlock for Vertical<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.blocks
            .iter()
            .map(|block| block.calculate_dimension(string_bounder))
            .reduce(XDimension2D::merge_top_bottom)
            .unwrap_or_default()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let total = self.calculate_dimension(string_bounder);
        let mut y = 0.0;
        for block in &self.blocks {
            let dimension = block.calculate_dimension(string_bounder);
            if let Some(back) = block.backcolor().filter(|back| !back.is_transparent()) {
                ug.translated(0.0, y)
                    .with_color(back.clone())
                    .with_backcolor(back)
                    .draw(&UShape::Rectangle(URectangle::new(
                        total.width,
                        dimension.height,
                    )));
            }
            let dx = match self.alignment {
                HorizontalAlignment::Left => 0.0,
                HorizontalAlignment::Center => (total.width - dimension.width) / 2.0,
                HorizontalAlignment::Right => total.width - dimension.width,
            };
            block.draw_u(&ug.translated(dx, y));
            y += dimension.height;
        }
    }
}

/// A block that asks to be drawn on a colour; the block stacking it paints it.
pub struct WithBackcolor<T> {
    inner: T,
    color: HColor,
}

impl<T: TextBlock> WithBackcolor<T> {
    pub fn new(inner: T, color: HColor) -> Self {
        Self { inner, color }
    }
}

impl<T: TextBlock> TextBlock for WithBackcolor<T> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.inner.calculate_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.inner.draw_u(ug);
    }

    fn backcolor(&self) -> Option<HColor> {
        Some(self.color.clone())
    }
}

/// Lines of text without creole markup, each at least 10 high (PlantUML's `TextBlockRaw`).
pub struct RawText {
    lines: Vec<String>,
    font: FontConfiguration,
}

impl RawText {
    /// An empty line is kept as a single space.
    pub fn new(
        lines: impl IntoIterator<Item = impl Into<String>>,
        font: FontConfiguration,
    ) -> Self {
        let lines = lines
            .into_iter()
            .map(|line| {
                let line = line.into();
                if line.is_empty() {
                    " ".to_owned()
                } else {
                    line
                }
            })
            .collect();
        Self { lines, font }
    }

    fn line_dimension(&self, line: &str, string_bounder: &dyn StringBounder) -> XDimension2D {
        let font = self.font.font();
        let rect = string_bounder.calculate_dimension(&font, line);
        let width = if line.contains(['\t', BLOCK_E1_REAL_TABULATION]) {
            self.layout(line, string_bounder, |_, _| {})
        } else {
            rect.width
        };
        let space_below = f64::from(self.font.space().abs());
        XDimension2D::new(width, rect.height.max(10.0) + space_below)
    }

    /// Calls `visit` with each piece between tabulations and its x offset; returns the total width.
    fn layout(
        &self,
        line: &str,
        string_bounder: &dyn StringBounder,
        mut visit: impl FnMut(&str, f64),
    ) -> f64 {
        let font = self.font.font();
        let tab_size = string_bounder.calculate_dimension(&font, "        ").width;
        let mut x = 0.0;
        for piece in line.split_inclusive(['\t', BLOCK_E1_REAL_TABULATION]) {
            let (text, tabulated) = match piece.strip_suffix(['\t', BLOCK_E1_REAL_TABULATION]) {
                Some(text) => (text, true),
                None => (piece, false),
            };
            if !text.is_empty() {
                visit(text, x);
                x += string_bounder.calculate_dimension(&font, text).width;
            }
            if tabulated {
                x += tab_size - x % tab_size;
            }
        }
        x
    }
}

impl TextBlock for RawText {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.lines
            .iter()
            .map(|line| self.line_dimension(line, string_bounder))
            .fold(XDimension2D::default(), XDimension2D::merge_top_bottom)
    }

    /// Text is placed by its top plus the font size, and moved by the font position's space.
    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let ug = ug.with_color(self.font.color().clone());
        let font_size = self.font.font().size_2d();
        let space = f64::from(self.font.space());
        let mut y = 0.0;
        for line in &self.lines {
            self.layout(line, string_bounder, |piece, x| {
                let text = UText::new(piece, self.font.clone());
                ug.translated(x, y + font_size + space)
                    .draw(&UShape::Text(text));
            });
            y += self.line_dimension(line, string_bounder).height;
        }
    }
}
