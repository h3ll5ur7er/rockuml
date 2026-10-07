//! Text blocks that frame, pad, stack or mark other text blocks.

use super::font::{FontConfiguration, StringBounder, UFont};
use super::geom::{ClockwiseTopRightBottomLeft, XDimension2D, XRectangle2D};
use super::group::UGroup;
use super::shape::{UCenteredCharacter, UEllipse, URectangle, UShape, UText};
use super::stencil::UHorizontalLine;
use super::ugraphic::{UGraphic, UStroke};
use super::{HorizontalAlignment, TextBlock, layout_tabulated};
use crate::color::HColor;
use crate::jaws::BLOCK_E1_REAL_TABULATION;

/// A block inside padding, with a border and background drawn around both.
pub(crate) struct TextBlockBordered<T> {
    inner: T,
    stroke: UStroke,
    border: HColor,
    background: HColor,
    corner: f64,
    padding: ClockwiseTopRightBottomLeft,
}

impl<T: TextBlock> TextBlockBordered<T> {
    pub(crate) fn new(
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

impl<T: TextBlock> TextBlock for TextBlockBordered<T> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.frame(string_bounder).delta(1.0, 1.0)
    }

    fn draw_u(&self, ug: &UGraphic) {
        // PlantUML compares gradients by identity, so a gradient never matches the image's background.
        let same_as_image = &self.background == ug.default_background()
            && !matches!(self.background, HColor::Gradient(_));
        let background = if self.background.is_transparent() || same_as_image {
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
        self.inner.draw_in_padding(
            &ug.with_color(border)
                .translated(self.padding.left, self.padding.top),
            self.padding.left,
            self.padding.right,
        );
    }
}

/// A letter in a filled circle, like a stereotype's spot.
pub(crate) struct CircledCharacter {
    character: char,
    radius: f64,
    font: UFont,
    spot_back_color: Option<HColor>,
    /// The outline; the surface's colour without one.
    spot_border: Option<HColor>,
    font_color: HColor,
}

impl CircledCharacter {
    pub(crate) fn new(
        character: char,
        radius: f64,
        font: UFont,
        spot_back_color: Option<HColor>,
        font_color: HColor,
    ) -> Self {
        Self {
            character,
            radius,
            font,
            spot_back_color,
            spot_border: None,
            font_color,
        }
    }

    #[must_use]
    pub(crate) fn with_border(self, spot_border: HColor) -> Self {
        Self {
            spot_border: Some(spot_border),
            ..self
        }
    }
}

impl TextBlock for CircledCharacter {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(2.0 * self.radius, 2.0 * self.radius)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let mut circle = match &self.spot_border {
            Some(border) => ug.with_color(border.clone()),
            None => ug.clone(),
        };
        if let Some(color) = &self.spot_back_color {
            circle = circle.with_backcolor(color.clone());
        }
        circle.draw(&UShape::Ellipse(UEllipse::new(
            2.0 * self.radius,
            2.0 * self.radius,
        )));
        circle
            .with_color(self.font_color.clone())
            .translated(self.radius, self.radius)
            .draw(&UShape::CenteredCharacter(UCenteredCharacter {
                character: self.character,
                font: self.font.clone(),
            }));
    }
}

/// A block with a sprite, like a stereotype's spot, before it.
pub(crate) struct TextBlockSprited<S, T> {
    sprite: S,
    parent: T,
}

impl<S: TextBlock, T: TextBlock> TextBlockSprited<S, T> {
    const MARGIN: f64 = 6.0;

    pub(crate) fn new(sprite: S, parent: T) -> Self {
        Self { sprite, parent }
    }

    fn sprite_width_and_margin(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.sprite.calculate_dimension(string_bounder).width + Self::MARGIN
    }
}

impl<S: TextBlock, T: TextBlock> TextBlock for TextBlockSprited<S, T> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let sprite_height = self.sprite.calculate_dimension(string_bounder).height;
        let dimension = self.parent.calculate_dimension(string_bounder);
        XDimension2D::new(
            dimension.width + self.sprite_width_and_margin(string_bounder),
            sprite_height.max(dimension.height),
        )
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.draw_in_padding(ug, 0.0, 0.0);
    }

    fn draw_in_padding(&self, ug: &UGraphic, left: f64, right: f64) {
        self.sprite.draw_u(ug);
        let dx = self.sprite_width_and_margin(ug.string_bounder());
        self.parent
            .draw_in_padding(&ug.translated(dx, 0.0), left, right);
    }
}

/// A block with empty space around it.
pub(crate) struct TextBlockMarged<T> {
    inner: T,
    margin: ClockwiseTopRightBottomLeft,
}

impl<T: TextBlock> TextBlockMarged<T> {
    pub(crate) fn new(inner: T, margin: ClockwiseTopRightBottomLeft) -> Self {
        Self { inner, margin }
    }
}

impl<T: TextBlock> TextBlock for TextBlockMarged<T> {
    fn get_inner_position(
        &self,
        member: &str,
        string_bounder: &dyn StringBounder,
    ) -> Option<XRectangle2D> {
        self.inner
            .get_inner_position(member, string_bounder)
            .map(|parent| parent.translated(self.margin.left, self.margin.top))
    }

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
pub(crate) struct DecorateEntityImage<'a> {
    original: Box<dyn TextBlock + 'a>,
    top: Option<Decoration<'a>>,
    bottom: Option<Decoration<'a>>,
}

/// A block placed above or below another, drawn in its own group.
pub(crate) struct Decoration<'a> {
    pub block: Box<dyn TextBlock + 'a>,
    pub alignment: HorizontalAlignment,
    pub group: UGroup,
}

impl<'a> DecorateEntityImage<'a> {
    pub(crate) fn new(
        original: Box<dyn TextBlock + 'a>,
        top: Option<Decoration<'a>>,
        bottom: Option<Decoration<'a>>,
    ) -> Self {
        Self {
            original,
            top,
            bottom,
        }
    }
}

fn dimension_of(
    decoration: Option<&Decoration<'_>>,
    string_bounder: &dyn StringBounder,
) -> XDimension2D {
    decoration.map_or_else(XDimension2D::default, |decoration| {
        decoration.block.calculate_dimension(string_bounder)
    })
}

impl TextBlock for DecorateEntityImage<'_> {
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
        let draw_decoration = |decoration: &Decoration, y: f64| {
            let width = decoration.block.calculate_dimension(string_bounder).width;
            let x = decoration.alignment.offset(total.width, width);
            ug.start_group(&decoration.group);
            decoration.block.draw_u(&ug.translated(x, y));
            ug.close_group();
        };
        if let Some(top) = &self.top {
            draw_decoration(top, 0.0);
        }
        self.original
            .draw_u(&ug.translated((total.width - original.width) / 2.0, top_height));
        if let Some(bottom) = &self.bottom {
            draw_decoration(bottom, top_height + original.height);
        }
    }
}

/// Two blocks side by side, centred vertically (`TextBlockUtils.mergeLR`).
pub(crate) struct TextBlockHorizontal {
    pub left: Box<dyn TextBlock>,
    pub right: Box<dyn TextBlock>,
}

impl TextBlock for TextBlockHorizontal {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let left = self.left.calculate_dimension(string_bounder);
        let right = self.right.calculate_dimension(string_bounder);
        XDimension2D::new(left.width + right.width, left.height.max(right.height))
    }

    fn draw_u(&self, ug: &UGraphic) {
        let total = self.calculate_dimension(ug.string_bounder());
        let mut x = 0.0;
        for block in [&self.left, &self.right] {
            let dimension = block.calculate_dimension(ug.string_bounder());
            block.draw_u(&ug.translated(x, (total.height - dimension.height) / 2.0));
            x += dimension.width;
        }
    }
}

/// Blocks stacked top to bottom, each on the full width in its own background colour if it has one.
pub(crate) struct TextBlockVertical<'a> {
    blocks: Vec<Box<dyn TextBlock + 'a>>,
    alignment: HorizontalAlignment,
}

impl<'a> TextBlockVertical<'a> {
    pub(crate) fn new(
        blocks: Vec<Box<dyn TextBlock + 'a>>,
        alignment: HorizontalAlignment,
    ) -> Self {
        Self { blocks, alignment }
    }
}

impl TextBlock for TextBlockVertical<'_> {
    /// PlantUML leaves the blocks' alignment out.
    fn get_inner_position(
        &self,
        member: &str,
        string_bounder: &dyn StringBounder,
    ) -> Option<XRectangle2D> {
        let mut y = 0.0;
        for block in &self.blocks {
            if let Some(result) = block.get_inner_position(member, string_bounder) {
                return Some(result.translated(0.0, y));
            }
            y += block.calculate_dimension(string_bounder).height;
        }
        None
    }

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
            let dx = self.alignment.offset(total.width, dimension.width);
            block.draw_u(&ug.translated(dx, y));
            y += dimension.height;
        }
    }
}

/// A block that asks to be drawn on a colour; the block stacking it paints it.
pub(crate) struct WithBackcolor<T> {
    inner: T,
    color: HColor,
}

impl<T: TextBlock> WithBackcolor<T> {
    pub(crate) fn new(inner: T, color: HColor) -> Self {
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

/// Lines of text without creole markup, each at least 10 high.
pub(crate) struct TextBlockRaw {
    lines: Vec<String>,
    font: FontConfiguration,
}

impl TextBlockRaw {
    /// An empty line is kept as a single space.
    pub(crate) fn new(
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

    /// Tab stops are eight spaces apart.
    fn layout(
        &self,
        line: &str,
        string_bounder: &dyn StringBounder,
        visit: impl FnMut(&str, f64),
    ) -> f64 {
        let font = self.font.font();
        let width_of = |text: &str| string_bounder.calculate_dimension(&font, text).width;
        layout_tabulated(line, width_of("        "), width_of, visit)
    }
}

impl TextBlock for TextBlockRaw {
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

/// A block with a separator line across its top.
pub(crate) struct TextBlockLineBefore<'a> {
    pub block: Box<dyn TextBlock + 'a>,
    pub style: char,
    pub title: Option<Box<dyn TextBlock + 'a>>,
    pub thickness: f64,
}

impl TextBlockLineBefore<'_> {
    fn line(&self) -> UHorizontalLine<'_> {
        UHorizontalLine {
            style: self.style,
            title: self.title.as_deref(),
            default_thickness: self.thickness,
            skip: 1.0,
        }
    }
}

impl TextBlock for TextBlockLineBefore<'_> {
    fn get_inner_position(
        &self,
        member: &str,
        string_bounder: &dyn StringBounder,
    ) -> Option<XRectangle2D> {
        self.block.get_inner_position(member, string_bounder)
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let dimension = self.block.calculate_dimension(string_bounder);
        match &self.title {
            None => dimension,
            Some(title) => {
                let title = title.calculate_dimension(string_bounder);
                XDimension2D::new(
                    dimension.width.max(title.width + 8.0),
                    dimension.height.max(title.height),
                )
            }
        }
    }

    fn draw_u(&self, ug: &UGraphic) {
        if self.title.is_none() {
            ug.draw_horizontal_line(&self.line());
        }
        self.block.draw_u(ug);
        if self.title.is_some() {
            ug.draw_horizontal_line(&self.line());
        }
    }
}


/// A block under a separator with a title, which leaves room for half the title above and below the line.
pub(crate) struct TitledSeparator {
    pub block: Box<dyn TextBlock>,
    pub style: char,
    pub title: Box<dyn TextBlock>,
    pub thickness: f64,
    /// The room left of the block.
    pub margin_x: f64,
}

impl TitledSeparator {
    fn layout(&self, string_bounder: &dyn StringBounder) -> impl TextBlock + '_ {
        let half_title = self.title.calculate_dimension(string_bounder).height / 2.0;
        let margin = ClockwiseTopRightBottomLeft::top_right_bottom_left;
        let raw = TextBlockLineBefore {
            block: Box::new(TextBlockMarged::new(
                &*self.block,
                margin(half_title, 6.0, 4.0, self.margin_x),
            )),
            style: self.style,
            title: Some(Box::new(&*self.title)),
            thickness: self.thickness,
        };
        TextBlockMarged::new(raw, margin(half_title, 0.0, 0.0, 0.0))
    }
}

impl TextBlock for TitledSeparator {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.layout(string_bounder)
            .calculate_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.layout(ug.string_bounder()).draw_u(ug);
    }

    fn get_inner_position(
        &self,
        member: &str,
        string_bounder: &dyn StringBounder,
    ) -> Option<XRectangle2D> {
        self.layout(string_bounder)
            .get_inner_position(member, string_bounder)
    }
}
