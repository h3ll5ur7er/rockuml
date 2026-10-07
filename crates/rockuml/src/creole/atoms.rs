//! The atoms that are not text.

use std::rc::Rc;

use super::sheet_block::SheetBlock1;
use super::{Atom, Sheet};
use crate::color::HColor;
use crate::emoji::Emoji;
use crate::klimt::TextBlock;
use crate::klimt::blocks::TextBlockMarged;
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::shape::{UEllipse, URectangle, UShape};
use crate::klimt::sprite::Sprite;
use crate::klimt::stencil::UHorizontalLine;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::openiconic::{OpenIconic, OpenIconicBlock};

/// The mark in front of a `*` list item: a disc at the first level, a square below.
pub(super) struct Bullet {
    font: FontConfiguration,
    order: usize,
}

impl Bullet {
    pub(super) fn new(font: FontConfiguration, order: usize) -> Self {
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
pub(super) struct HorizontalLine {
    /// The character the line is drawn with.
    style: char,
    title: Option<SheetBlock1>,
}

impl HorizontalLine {
    pub(super) fn new(style: char, title: Option<Sheet>) -> Self {
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
                default_thickness: 1.0,
                skip: 0.0,
            });
    }
}

impl Atom for HorizontalLine {
    fn starting_altitude(&self, _string_bounder: &dyn StringBounder) -> f64 {
        0.0
    }
}

/// An atom with space above and below it, as tables and trees have.
pub(super) struct AtomWithMargin<A> {
    atom: A,
    top: f64,
    bottom: f64,
}

impl<A: Atom> AtomWithMargin<A> {
    pub(super) fn new(atom: A, top: f64, bottom: f64) -> Self {
        Self { atom, top, bottom }
    }
}

impl<A: Atom> TextBlock for AtomWithMargin<A> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.atom
            .calculate_dimension(string_bounder)
            .delta(0.0, self.top + self.bottom)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.atom.draw_u(&ug.translated(0.0, self.top));
    }
}

impl<A: Atom> Atom for AtomWithMargin<A> {
    fn starting_altitude(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.atom.starting_altitude(string_bounder)
    }
}

/// An `OpenIconic` icon (`<&heart>`), sized to the font and in its colour unless given one.
pub(super) struct AtomOpenIconic {
    open_iconic: OpenIconic,
    factor: f64,
    color: HColor,
}

impl AtomOpenIconic {
    pub(super) fn new(
        new_color: Option<HColor>,
        scale: f64,
        open_iconic: OpenIconic,
        font: &FontConfiguration,
    ) -> Self {
        Self {
            open_iconic,
            factor: scale * font.size_2d() / 12.0,
            color: new_color.unwrap_or_else(|| font.color().clone()),
        }
    }

    fn as_text_block(&self) -> TextBlockMarged<OpenIconicBlock<'_>> {
        TextBlockMarged::new(
            self.open_iconic
                .as_text_block(self.color.clone(), self.factor),
            ClockwiseTopRightBottomLeft {
                top: 0.0,
                right: 1.0,
                bottom: 0.0,
                left: 1.0,
            },
        )
    }
}

impl TextBlock for AtomOpenIconic {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.as_text_block().calculate_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.as_text_block().draw_u(ug);
    }
}

impl Atom for AtomOpenIconic {
    fn starting_altitude(&self, _string_bounder: &dyn StringBounder) -> f64 {
        -3.0 * self.factor
    }
}

/// An emoji (`<:smile:>`), sized to the font.
pub(super) struct AtomEmoji {
    emoji: &'static Emoji,
    factor: f64,
    color: Option<HColor>,
}

impl AtomEmoji {
    /// The font size at which an emoji is drawn 36 units square.
    const MAGIC: f64 = 24.0;

    pub(super) fn new(
        emoji: &'static Emoji,
        scale: f64,
        size_2d: f64,
        color: Option<HColor>,
    ) -> Self {
        Self {
            emoji,
            factor: scale * size_2d / Self::MAGIC,
            color,
        }
    }
}

impl TextBlock for AtomEmoji {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        let size = 36.0 * self.factor;
        XDimension2D::new(size, size)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.emoji.draw_u(ug, self.factor, self.color.as_ref());
    }
}

impl Atom for AtomEmoji {
    fn starting_altitude(&self, _string_bounder: &dyn StringBounder) -> f64 {
        -3.0 * self.factor
    }
}

/// A sprite (`<$name>`), in the text colour unless given one.
pub(super) struct AtomSprite {
    font_color: HColor,
    forced_color: Option<HColor>,
    scale: f64,
    sprite: Rc<dyn Sprite>,
    back_color: Option<HColor>,
}

impl AtomSprite {
    pub(super) fn new(
        font_color: HColor,
        forced_color: Option<HColor>,
        scale: f64,
        sprite: Rc<dyn Sprite>,
        back_color: Option<HColor>,
    ) -> Self {
        Self {
            font_color,
            forced_color,
            scale,
            sprite,
            back_color,
        }
    }

    fn as_text_block(&self, back_color: Option<&HColor>) -> Box<dyn TextBlock + '_> {
        self.sprite.as_text_block(
            &self.font_color,
            self.forced_color.as_ref(),
            self.scale,
            back_color,
        )
    }
}

impl TextBlock for AtomSprite {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.as_text_block(None).calculate_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.as_text_block(self.back_color.as_ref()).draw_u(ug);
    }
}

impl Atom for AtomSprite {
    fn starting_altitude(&self, _string_bounder: &dyn StringBounder) -> f64 {
        0.0
    }
}
