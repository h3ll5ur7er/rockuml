//! The pieces railroad diagrams are made of (PlantUML's `ETile` and its subclasses). Each tile has a track
//! running through it, `h1` below its top and `h2` above its bottom.

use super::curves::{Brace, CornerCurved, CornerType, Zigzag};
use crate::color::{HColor, XColor};
use crate::creole::Display;
use crate::direction::Direction;
use crate::ftile::vcompact::FloatingNote;
use crate::klimt::TextBlock;
use crate::klimt::blocks::TextBlockMarged;
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D, XPoint2D};
use crate::klimt::shape::{UEllipse, URectangle, USegment, UShape, UText};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::skin::SkinParam;
use crate::style::SName;

/// What tiles are drawn with: the `ebnf` element style's font and colours.
#[derive(Clone)]
pub(crate) struct TileStyle {
    pub font: FontConfiguration,
    pub line_color: HColor,
    pub background_color: HColor,
}

/// What a box holds, which decides its outline.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum BoxKind {
    /// A quoted string, in a plain rectangle.
    Terminal,
    /// `? … ?`, in a dashed rectangle.
    SpecialSequence,
    /// A rule's name or a character class, in a rounded filled rectangle.
    Literal,
}

pub(crate) enum ETile {
    Box(ETileBox),
    Concatenation(ETileConcatenation),
    Alternation(ETileAlternation),
    Optional2(ETileOptional2),
    OneOrMore(ETileOneOrMore),
    Not(ETileNot),
    Empty,
    LookAheadOrBehind(ETileLookAheadOrBehind),
    NamedGroup(ETileNamedGroup),
    RegexGroup(ETileRegexGroup),
    RegexGroupAllBut(ETileRegexGroupAllBut),
}

impl ETile {
    pub(crate) fn width(&self, sb: &dyn StringBounder) -> f64 {
        match self {
            Self::Box(tile) => tile.width(sb),
            Self::Concatenation(tile) => tile.width(sb),
            Self::Alternation(tile) => tile.width(sb),
            Self::Optional2(tile) => tile.width(sb),
            Self::OneOrMore(tile) => tile.width(sb),
            Self::Not(tile) => tile.width(sb),
            Self::Empty => 0.0,
            Self::LookAheadOrBehind(tile) => tile.width(sb),
            Self::NamedGroup(tile) => tile.width(sb),
            Self::RegexGroup(tile) => tile.width(sb),
            Self::RegexGroupAllBut(tile) => tile.width(sb),
        }
    }

    pub(crate) fn h1(&self, sb: &dyn StringBounder) -> f64 {
        match self {
            Self::Box(tile) => tile.h1(sb),
            Self::Concatenation(tile) => tile.h1(sb),
            Self::Alternation(tile) => tile.h1(sb),
            Self::Optional2(tile) => tile.h1(sb),
            Self::OneOrMore(tile) => tile.h1(sb),
            Self::Not(tile) => tile.orig.h1(sb),
            Self::Empty => 0.0,
            Self::LookAheadOrBehind(tile) => tile.h1(sb),
            Self::NamedGroup(tile) => tile.h1(sb),
            Self::RegexGroup(tile) => tile.half_height(sb),
            Self::RegexGroupAllBut(tile) => tile.half_height(sb),
        }
    }

    pub(crate) fn h2(&self, sb: &dyn StringBounder) -> f64 {
        match self {
            Self::Box(tile) => tile.h2(sb),
            Self::Concatenation(tile) => tile.h2(sb),
            Self::Alternation(tile) => tile.h2(sb),
            Self::Optional2(tile) => tile.h2(sb),
            Self::OneOrMore(tile) => tile.orig.h2(sb),
            Self::Not(tile) => tile.orig.h2(sb),
            Self::Empty => 0.0,
            Self::LookAheadOrBehind(tile) => tile.h2(sb),
            Self::NamedGroup(tile) => tile.h2(sb),
            Self::RegexGroup(tile) => tile.half_height(sb),
            Self::RegexGroupAllBut(tile) => tile.half_height(sb),
        }
    }

    /// `orig` made optional: the track may go around it, below it.
    pub(crate) fn optional(orig: ETile) -> Self {
        Self::Optional2(ETileOptional2::new(orig))
    }

    /// `orig` repeated, its track looping back above it, with `loop_label` over the loop.
    pub(crate) fn one_or_more(
        orig: ETile,
        loop_label: Option<(String, FontConfiguration)>,
    ) -> Self {
        Self::OneOrMore(ETileOneOrMore::new(orig, loop_label))
    }

    /// `orig` any number of times (`ETileZeroOrMore`).
    pub(crate) fn zero_or_more(orig: ETile) -> Self {
        Self::optional(Self::one_or_more(orig, None))
    }

    /// `arg2 | arg1`, where `arg1` was on top of the engine's stack: an alternation on either side takes the
    /// other in (the engines' `alternation`).
    pub(crate) fn alternation(mut arg1: ETile, mut arg2: ETile) -> Self {
        if matches!(arg1, Self::Alternation(_)) {
            arg1.push(arg2);
            arg1
        } else if matches!(arg2, Self::Alternation(_)) {
            arg2.push(arg1);
            arg2
        } else {
            Self::Alternation(ETileAlternation::of(arg2, arg1))
        }
    }

    /// The first of a concatenation or an alternation, the others moving after it (`push`). Other tiles
    /// hold no list, and PlantUML throws.
    pub(crate) fn push(&mut self, tile: ETile) {
        match self {
            Self::Concatenation(concatenation) => concatenation.tiles.insert(0, tile),
            Self::Alternation(alternation) => alternation.tiles.insert(0, tile),
            _ => unreachable!("only lists of tiles take more tiles"),
        }
    }

    /// Boxes and optional parts show comments; PlantUML ignores them on other tiles.
    pub(crate) fn add_comment_above(&mut self, comment: &str, skin_param: &SkinParam) {
        match self {
            Self::Box(tile) => tile.comment_above = Some(opale_note(comment, skin_param)),
            Self::Optional2(tile) => {
                tile.note_above = Some(marged_note(comment, skin_param, 0.0, 10.0));
            }
            _ => {}
        }
    }

    pub(crate) fn add_comment_below(&mut self, comment: &str, skin_param: &SkinParam) {
        match self {
            Self::Box(tile) => tile.comment_below = Some(opale_note(comment, skin_param)),
            Self::Optional2(tile) => {
                tile.note_below = Some(marged_note(comment, skin_param, 10.0, 0.0));
            }
            _ => {}
        }
    }

    /// The count a box gives a repetition `3 * x` written before it.
    pub(crate) fn repetition_label(&self) -> &str {
        match self {
            Self::Box(tile) => &tile.value,
            _ => "?",
        }
    }
}

impl TextBlock for ETile {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(self.width(sb), self.h1(sb) + self.h2(sb))
    }

    fn draw_u(&self, ug: &UGraphic) {
        match self {
            Self::Box(tile) => tile.draw_u(ug),
            Self::Concatenation(tile) => tile.draw_u(ug),
            Self::Alternation(tile) => tile.draw_u(ug),
            Self::Optional2(tile) => tile.draw_u(ug),
            Self::OneOrMore(tile) => tile.draw_u(ug),
            Self::Not(tile) => tile.draw_u(ug),
            Self::Empty => {}
            Self::LookAheadOrBehind(tile) => tile.draw_u(ug),
            Self::NamedGroup(tile) => tile.draw_u(ug),
            Self::RegexGroup(tile) => tile.draw_u(ug),
            Self::RegexGroupAllBut(tile) => tile.draw_u(ug),
        }
    }
}

/// A note above or below a box, reaching towards it.
fn opale_note(comment: &str, skin_param: &SkinParam) -> FloatingNote {
    FloatingNote::create(&Display::with_newlines(comment), skin_param, SName::Ebnf)
}

/// A note with room above and below it.
fn marged_note(
    comment: &str,
    skin_param: &SkinParam,
    top: f64,
    bottom: f64,
) -> TextBlockMarged<FloatingNote> {
    TextBlockMarged::new(
        opale_note(comment, skin_param),
        ClockwiseTopRightBottomLeft::top_right_bottom_left(top, 0.0, bottom, 0.0),
    )
}

fn height_of(note: Option<&impl TextBlock>, sb: &dyn StringBounder) -> f64 {
    note.map_or(0.0, |note| note.calculate_dimension(sb).height)
}

fn text_dimension(font: &FontConfiguration, text: &str, sb: &dyn StringBounder) -> XDimension2D {
    sb.calculate_dimension(&font.font(), text)
}

fn descent(font: &FontConfiguration, text: &str, sb: &dyn StringBounder) -> f64 {
    sb.descent(&font.font(), text)
}

fn draw_text(ug: &UGraphic, text: &str, font: &FontConfiguration, x: f64, y: f64) {
    ug.translated(x, y)
        .draw(&UShape::Text(UText::new(text, font.clone())));
}

fn draw_hline(ug: &UGraphic, y: f64, x1: f64, x2: f64) {
    ug.translated(x1, y).draw(&UShape::Line {
        dx: x2 - x1,
        dy: 0.0,
    });
}

fn draw_vline(ug: &UGraphic, x: f64, y1: f64, y2: f64) {
    ug.translated(x, y1).draw(&UShape::Line {
        dx: 0.0,
        dy: y2 - y1,
    });
}

/// The line with an arrow `coef` along it, if it is longer than `minimum_size_for_arrow`.
fn draw_hline_directed(
    ug: &UGraphic,
    y: f64,
    x1: f64,
    x2: f64,
    coef: f64,
    minimum_size_for_arrow: f64,
) {
    draw_hline(ug, y, x1, x2);
    if x2 > x1 + minimum_size_for_arrow {
        draw_arrow(ug, x1 * (1.0 - coef) + x2 * coef - 2.0, y, ARROW_TO_RIGHT);
    }
}

fn draw_hline_anti_directed(ug: &UGraphic, y: f64, x1: f64, x2: f64, coef: f64) {
    draw_hline(ug, y, x1, x2);
    draw_arrow(ug, x1 * (1.0 - coef) + x2 * coef - 2.0, y, ARROW_TO_LEFT);
}

const ARROW_TO_LEFT: [(f64, f64); 3] = [(0.0, -3.0), (-6.0, 0.0), (0.0, 3.0)];
const ARROW_TO_RIGHT: [(f64, f64); 3] = [(0.0, -3.0), (6.0, 0.0), (0.0, 3.0)];

/// An arrow head filled with the line colour (`CopyForegroundColorToBackgroundColor`).
fn draw_arrow(ug: &UGraphic, x: f64, y: f64, points: [(f64, f64); 3]) {
    let mut segments = vec![USegment::MoveTo(0.0, 0.0)];
    segments.extend(points.iter().map(|&(x, y)| USegment::LineTo(x, y)));
    segments.push(USegment::LineTo(0.0, 0.0));
    ug.with_backcolor(ug.param().color.clone())
        .translated(x, y)
        .draw(&UShape::path(segments));
}

fn corner(corner_type: CornerType, delta: f64, ug: &UGraphic, x: f64, y: f64) {
    CornerCurved::new(corner_type, delta).draw_u(&ug.translated(x, y));
}

/// A rule's name, a string or a character class in a box (`ETileBox`).
pub(crate) struct ETileBox {
    value: String,
    kind: BoxKind,
    style: TileStyle,
    comment_above: Option<FloatingNote>,
    comment_below: Option<FloatingNote>,
}

impl ETileBox {
    pub(crate) fn new(value: String, kind: BoxKind, style: TileStyle) -> Self {
        Self {
            value,
            kind,
            style,
            comment_above: None,
            comment_below: None,
        }
    }

    pub(crate) fn kind(&self) -> BoxKind {
        self.kind
    }

    /// A box holding this one's text then `other`'s (`mergeWith`).
    pub(crate) fn merge_with(&self, other: &ETileBox) -> Self {
        Self::new(
            format!("{}{}", self.value, other.value),
            self.kind,
            self.style.clone(),
        )
    }

    fn text_dim(&self, sb: &dyn StringBounder) -> XDimension2D {
        text_dimension(&self.style.font, &self.value, sb)
    }

    fn pure_h1(&self, sb: &dyn StringBounder) -> f64 {
        f64::midpoint(self.text_dim(sb).height, 10.0)
    }

    fn note_room(note: Option<&FloatingNote>, sb: &dyn StringBounder) -> f64 {
        note.map_or(0.0, |note| note.calculate_dimension(sb).height + 20.0)
    }

    fn h1(&self, sb: &dyn StringBounder) -> f64 {
        self.pure_h1(sb) + Self::note_room(self.comment_above.as_ref(), sb)
    }

    fn h2(&self, sb: &dyn StringBounder) -> f64 {
        self.pure_h1(sb) + Self::note_room(self.comment_below.as_ref(), sb)
    }

    fn width(&self, sb: &dyn StringBounder) -> f64 {
        let mut width = self.text_dim(sb).width + 10.0;
        for note in [&self.comment_above, &self.comment_below]
            .into_iter()
            .flatten()
        {
            width = width.max(note.calculate_dimension(sb).width);
        }
        width
    }

    fn draw_u(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let dim = XDimension2D::new(self.width(sb), self.h1(sb) + self.h2(sb));
        let dim_text = self.text_dim(sb);
        let dim_box = dim_text.delta(10.0, 10.0);
        let posy = Self::note_room(self.comment_above.as_ref(), sb);
        let posx_box = (dim.width - dim_box.width) / 2.0;
        let rect = URectangle::new(dim_box.width, dim_box.height);
        let at_box = ug
            .translated(posx_box, posy)
            .with_color(self.style.line_color.clone());
        match self.kind {
            BoxKind::Terminal => at_box
                .with_stroke(UStroke::with_thickness(0.5))
                .draw(&UShape::Rectangle(rect)),
            BoxKind::SpecialSequence => at_box
                .with_stroke(UStroke {
                    dash_visible: 5.0,
                    dash_space: 5.0,
                    thickness: 1.0,
                })
                .draw(&UShape::Rectangle(rect)),
            BoxKind::Literal => at_box
                .with_backcolor(self.style.background_color.clone())
                .with_stroke(UStroke::with_thickness(1.5))
                .draw(&UShape::Rectangle(rect.rounded(10.0))),
        }
        let descent = descent(&self.style.font, &self.value, sb);
        draw_text(
            ug,
            &self.value,
            &self.style.font,
            5.0 + posx_box,
            posy + 5.0 + dim_text.height - descent,
        );
        if let Some(note) = &self.comment_above {
            let note_dim = note.calculate_dimension(sb);
            let pos = note_dim.width * 0.5;
            note.draw_opale(
                &ug.translated((dim.width - note_dim.width) / 2.0, 0.0),
                Direction::Down,
                XPoint2D::new(pos, note_dim.height),
                XPoint2D::new(pos, 20.0 + note_dim.height),
            );
        }
        if let Some(note) = &self.comment_below {
            let note_dim = note.calculate_dimension(sb);
            let pos = note_dim.width * 0.5;
            note.draw_opale(
                &ug.translated(
                    (dim.width - note_dim.width) / 2.0,
                    dim.height - note_dim.height,
                ),
                Direction::Up,
                XPoint2D::new(pos, 0.0),
                XPoint2D::new(pos, -20.0),
            );
        }
        if posx_box > 0.0 {
            let h1 = self.h1(sb);
            draw_hline_directed(ug, h1, 0.0, posx_box, 0.5, 25.0);
            draw_hline_directed(ug, h1, posx_box + dim_box.width, dim.width, 0.5, 25.0);
        }
    }
}

/// Tiles one after the other (`ETileConcatenation`).
#[derive(Default)]
pub(crate) struct ETileConcatenation {
    tiles: Vec<ETile>,
}

impl ETileConcatenation {
    const MARGINX: f64 = 20.0;

    pub(crate) fn of(first: ETile, second: ETile) -> Self {
        Self {
            tiles: vec![first, second],
        }
    }

    pub(crate) fn push_last(&mut self, tile: ETile) {
        self.tiles.push(tile);
    }

    pub(crate) fn first_mut(&mut self) -> &mut ETile {
        &mut self.tiles[0]
    }

    pub(crate) fn first(&self) -> &ETile {
        &self.tiles[0]
    }

    fn h1(&self, sb: &dyn StringBounder) -> f64 {
        self.tiles
            .iter()
            .fold(0.0, |result, tile| result.max(tile.h1(sb)))
    }

    fn h2(&self, sb: &dyn StringBounder) -> f64 {
        self.tiles
            .iter()
            .fold(0.0, |result, tile| result.max(tile.h2(sb)))
    }

    fn width(&self, sb: &dyn StringBounder) -> f64 {
        let mut width = 0.0;
        for (i, tile) in self.tiles.iter().enumerate() {
            width += tile.width(sb);
            if i != self.tiles.len() - 1 {
                width += Self::MARGINX;
            }
        }
        width
    }

    fn draw_u(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let full_line_pos = self.h1(sb);
        let mut x = 0.0;
        draw_hline(ug, full_line_pos, 0.0, x);
        for (i, tile) in self.tiles.iter().enumerate() {
            let line_pos = tile.h1(sb);
            tile.draw_u(&ug.translated(x, full_line_pos - line_pos));
            x += tile.calculate_dimension(sb).width;
            if i != self.tiles.len() - 1 {
                draw_hline_directed(ug, full_line_pos, x, x + Self::MARGINX, 0.5, 25.0);
                x += Self::MARGINX;
            }
        }
    }
}

/// Tiles one below the other, the track forking to each (`ETileAlternation`).
pub(crate) struct ETileAlternation {
    tiles: Vec<ETile>,
}

impl ETileAlternation {
    const MARGINX: f64 = 12.0;

    pub(crate) fn of(first: ETile, second: ETile) -> Self {
        Self {
            tiles: vec![first, second],
        }
    }

    fn h1(&self, sb: &dyn StringBounder) -> f64 {
        self.tiles[0].h1(sb)
    }

    fn h2(&self, sb: &dyn StringBounder) -> f64 {
        let mut height = self.tiles[0].h2(sb);
        for tile in &self.tiles[1..] {
            height += tile.h1(sb);
            height += tile.h2(sb);
            height += 10.0;
        }
        height
    }

    fn width(&self, sb: &dyn StringBounder) -> f64 {
        let width = self.tiles.iter().fold(0.0, |width: f64, tile| {
            width.max(tile.calculate_dimension(sb).width)
        });
        width + 2.0 * 2.0 * Self::MARGINX
    }

    fn draw_u(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let marginx = Self::MARGINX;
        let mut y = 0.0;
        let mut last_line_pos = 0.0;
        let a = 0.0;
        let b = a + marginx;
        let c = b + marginx;
        let r = self.width(sb);
        let q = r - marginx;
        let p = q - marginx;
        let line_pos = self.h1(sb);
        let count = self.tiles.len();
        for (i, tile) in self.tiles.iter().enumerate() {
            let dim = tile.calculate_dimension(sb);
            last_line_pos = y + tile.h1(sb);
            tile.draw_u(&ug.translated(c, y));
            if i == 0 {
                draw_hline(ug, last_line_pos, a, c);
                draw_hline(ug, last_line_pos, c + dim.width, r);
            } else if i < count - 1 {
                corner(CornerType::SW, marginx, ug, b, last_line_pos);
                draw_hline_directed(ug, last_line_pos, c + dim.width, p, 0.5, 25.0);
                corner(CornerType::SE, marginx, ug, q, last_line_pos);
            } else {
                draw_hline_directed(ug, last_line_pos, c + dim.width, p, 0.5, 25.0);
            }
            y += dim.height + 10.0;
        }
        let height42 = last_line_pos - line_pos;
        let ug_b = ug.translated(b, line_pos);
        let ug_q = ug.translated(q, line_pos);
        corner(CornerType::SW, marginx, &ug_b, 0.0, height42);
        draw_vline(&ug_b, 0.0, marginx, height42 - marginx);
        corner(CornerType::NE, marginx, &ug_b, 0.0, 0.0);
        corner(CornerType::SE, marginx, &ug_q, 0.0, height42);
        draw_vline(&ug_q, 0.0, marginx, height42 - marginx);
        corner(CornerType::NW, marginx, &ug_q, 0.0, 0.0);
    }
}

/// A tile the track may go around, below it (`ETileOptional2`).
pub(crate) struct ETileOptional2 {
    orig: Box<ETile>,
    note_above: Option<TextBlockMarged<FloatingNote>>,
    note_below: Option<TextBlockMarged<FloatingNote>>,
}

impl ETileOptional2 {
    const DELTAX: f64 = 24.0;
    const DELTAY: f64 = 20.0;

    fn new(orig: ETile) -> Self {
        Self {
            orig: Box::new(orig),
            note_above: None,
            note_below: None,
        }
    }

    fn h1(&self, sb: &dyn StringBounder) -> f64 {
        10.0 + height_of(self.note_above.as_ref(), sb)
    }

    fn delta_y(&self, sb: &dyn StringBounder) -> f64 {
        Self::DELTAY + height_of(self.note_above.as_ref(), sb)
    }

    fn h2(&self, sb: &dyn StringBounder) -> f64 {
        10.0 + self.orig.h1(sb) + self.orig.h2(sb) + height_of(self.note_below.as_ref(), sb)
    }

    fn width(&self, sb: &dyn StringBounder) -> f64 {
        self.orig.width(sb) + 2.0 * Self::DELTAX
    }

    fn draw_u(&self, ug: &UGraphic) {
        const CORNER: f64 = 12.0;
        let sb = ug.string_bounder();
        let dim = XDimension2D::new(self.width(sb), self.h1(sb) + self.h2(sb));
        let line_pos = self.h1(sb);
        draw_hline_directed(ug, line_pos, 0.0, dim.width, 0.4, 25.0);
        let zigzag = Zigzag::new(
            9.0,
            2.0 * CORNER,
            self.delta_y(sb) + self.orig.h1(sb) - line_pos,
        );
        ug.translated(0.0, line_pos).draw(&zigzag.path_down());
        ug.translated(dim.width - 2.0 * CORNER, line_pos)
            .draw(&zigzag.path_up());
        self.orig
            .draw_u(&ug.translated(Self::DELTAX, self.delta_y(sb)));
        if let Some(note) = &self.note_above {
            let pos2 = (dim.width - note.calculate_dimension(sb).width) / 2.0;
            note.draw_u(&ug.translated(pos2, 0.0));
        }
        if let Some(note) = &self.note_below {
            let dim_below = note.calculate_dimension(sb);
            let pos2 = (dim.width - dim_below.width) / 2.0;
            note.draw_u(&ug.translated(pos2, dim.height - dim_below.height));
        }
    }
}

/// A tile the track may loop back over, above it, with the count of a repetition (`ETileOneOrMore`).
pub(crate) struct ETileOneOrMore {
    orig: Box<ETile>,
    loop_label: Option<(String, FontConfiguration)>,
}

impl ETileOneOrMore {
    const DELTAX: f64 = 15.0;
    const DELTAY: f64 = 12.0;

    fn new(orig: ETile, loop_label: Option<(String, FontConfiguration)>) -> Self {
        Self {
            orig: Box::new(orig),
            loop_label,
        }
    }

    fn brace_height(&self) -> f64 {
        if self.loop_label.is_some() { 15.0 } else { 0.0 }
    }

    fn h1(&self, sb: &dyn StringBounder) -> f64 {
        Self::DELTAY + self.orig.h1(sb) + self.brace_height()
    }

    fn width(&self, sb: &dyn StringBounder) -> f64 {
        self.orig.width(sb) + 2.0 * Self::DELTAX
    }

    fn draw_u(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let width = self.width(sb);
        let h1 = self.h1(sb);
        let brace = self.brace_height();
        corner(CornerType::SW, 8.0, ug, 8.0, h1);
        draw_vline(ug, 8.0, 8.0 + 5.0 + brace, h1 - 8.0);
        corner(CornerType::NW, 8.0, ug, 8.0, 5.0 + brace);
        draw_hline_anti_directed(ug, 5.0 + brace, Self::DELTAX, width - Self::DELTAX, 0.6);
        corner(CornerType::SE, 8.0, ug, width - 8.0, h1);
        draw_vline(ug, width - 8.0, 8.0 + 5.0 + brace, h1 - 8.0);
        corner(CornerType::NE, 8.0, ug, width - 8.0, 5.0 + brace);
        draw_hline(ug, h1, 0.0, Self::DELTAX);
        draw_hline(ug, h1, width - Self::DELTAX, width);
        self.orig
            .draw_u(&ug.translated(Self::DELTAX, Self::DELTAY + brace));
        if let Some((label, font)) = &self.loop_label {
            Brace::new(width).draw_u(&ug.translated(0.0, 10.0));
            let dim_text = text_dimension(font, label, sb);
            let descent = descent(font, label, sb);
            draw_text(ug, label, font, (width - dim_text.width) / 2.0, descent);
        }
    }
}

/// A tile that must not match, after `NOT` (`ETileNot`).
pub(crate) struct ETileNot {
    orig: Box<ETile>,
    font: FontConfiguration,
}

impl ETileNot {
    const MARGIN_X1: f64 = 3.0;
    const MARGIN_X2: f64 = 8.0;
    const TEXT: &str = "NOT";

    pub(crate) fn new(orig: ETile, font: FontConfiguration) -> Self {
        Self {
            orig: Box::new(orig),
            font,
        }
    }

    fn width(&self, sb: &dyn StringBounder) -> f64 {
        self.orig.width(sb) + text_dimension(&self.font, Self::TEXT, sb).width + Self::MARGIN_X2
    }

    fn draw_u(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let dim_text = text_dimension(&self.font, Self::TEXT, sb);
        draw_text(
            ug,
            Self::TEXT,
            &self.font,
            Self::MARGIN_X1,
            self.orig.h1(sb) + descent(&self.font, Self::TEXT, sb) + 1.0,
        );
        self.orig
            .draw_u(&ug.translated(dim_text.width + Self::MARGIN_X2, 0.0));
    }
}

/// A rule's track between a start circle and an end circle (`ETileWithCircles`).
pub(crate) struct ETileWithCircles {
    orig: ETile,
    line_color: HColor,
}

impl ETileWithCircles {
    const SIZE: f64 = 8.0;
    const DELTAX: f64 = 30.0;

    pub(crate) fn new(orig: ETile, line_color: HColor) -> Self {
        Self { orig, line_color }
    }
}

impl TextBlock for ETileWithCircles {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> XDimension2D {
        let orig = self.orig.calculate_dimension(sb);
        XDimension2D::new(orig.width + 2.0 * Self::DELTAX, orig.height)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let line_pos = self.orig.h1(sb);
        let full_width = self.calculate_dimension(sb).width;
        let ug = ug
            .with_color(self.line_color.clone())
            .with_stroke(UStroke::with_thickness(1.5));
        self.orig.draw_u(&ug.translated(Self::DELTAX, 0.0));
        let circle = UShape::Ellipse(UEllipse::new(Self::SIZE, Self::SIZE));
        ug.with_stroke(UStroke::with_thickness(2.0))
            .translated(0.0, line_pos - Self::SIZE / 2.0)
            .draw(&circle);
        ug.with_stroke(UStroke::SIMPLE)
            .with_backcolor(self.line_color.clone())
            .translated(full_width - Self::SIZE / 2.0, line_pos - Self::SIZE / 2.0)
            .draw(&circle);
        draw_hline_directed(&ug, line_pos, Self::SIZE, Self::DELTAX, 0.5, 25.0);
        draw_hline_directed(
            &ug,
            line_pos,
            full_width - Self::DELTAX,
            full_width - Self::SIZE / 2.0,
            0.5,
            25.0,
        );
    }
}

/// A part that must or must not come before or after, in a dashed frame (`ETileLookAheadOrBehind`).
pub(crate) struct ETileLookAheadOrBehind {
    orig: Box<ETile>,
    style: TileStyle,
    sup_text: String,
}

impl ETileLookAheadOrBehind {
    const DELTAX1: f64 = 4.0;
    const DELTAX2: f64 = 8.0;
    const DELTAY: f64 = 6.0;

    pub(crate) fn new(orig: ETile, style: TileStyle, sup_text: String) -> Self {
        Self {
            orig: Box::new(orig),
            style,
            sup_text,
        }
    }

    fn sup_width(&self, sb: &dyn StringBounder) -> f64 {
        text_dimension(&self.style.font, &self.sup_text, sb).width
    }

    fn h1(&self, sb: &dyn StringBounder) -> f64 {
        Self::DELTAY + self.orig.h1(sb)
    }

    fn h2(&self, sb: &dyn StringBounder) -> f64 {
        self.orig.h2(sb) + Self::DELTAY
    }

    fn width(&self, sb: &dyn StringBounder) -> f64 {
        self.orig.width(sb) + Self::DELTAX1 + Self::DELTAX2 + self.sup_width(sb)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let rect = URectangle::new(self.width(sb), self.h1(sb) + self.h2(sb)).rounded(30.0);
        ug.with_color(self.style.line_color.clone())
            .with_stroke(UStroke {
                dash_visible: 2.0,
                dash_space: 3.0,
                thickness: 1.0,
            })
            .draw(&UShape::Rectangle(rect));
        let pos_text = self.h1(sb) + descent(&self.style.font, &self.sup_text, sb);
        draw_text(ug, &self.sup_text, &self.style.font, 4.0, 2.0 + pos_text);
        self.orig
            .draw_u(&ug.translated(Self::DELTAX1 + self.sup_width(sb), Self::DELTAY));
    }
}

/// A named capturing group, on a tinted background under its name (`ETileNamedGroup`).
pub(crate) struct ETileNamedGroup {
    orig: Box<ETile>,
    font: FontConfiguration,
    group_name: String,
}

impl ETileNamedGroup {
    const DELTAX: f64 = 10.0;
    const DELTAY1: f64 = 10.0;
    const DELTAY2: f64 = 10.0;
    const BACKGROUND: HColor = HColor::Simple(XColor::rgb(0xE8, 0xE8, 0xFF));

    pub(crate) fn new(orig: ETile, font: FontConfiguration, group_name: String) -> Self {
        Self {
            orig: Box::new(orig),
            font,
            group_name,
        }
    }

    fn h1(&self, sb: &dyn StringBounder) -> f64 {
        Self::DELTAY1 + self.orig.h1(sb)
    }

    fn h2(&self, sb: &dyn StringBounder) -> f64 {
        self.orig.h2(sb) + Self::DELTAY2
    }

    fn width(&self, sb: &dyn StringBounder) -> f64 {
        self.orig.width(sb) + 2.0 * Self::DELTAX
    }

    fn draw_u(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let dim = XDimension2D::new(self.width(sb), self.h1(sb) + self.h2(sb));
        let ug_back = ug
            .with_color(Self::BACKGROUND)
            .with_backcolor(Self::BACKGROUND);
        ug_back.draw(&UShape::Rectangle(URectangle::new(dim.width, dim.height)));
        let dim_text = text_dimension(&self.font, &self.group_name, sb);
        ug_back
            .translated(0.0, -dim_text.height)
            .draw(&UShape::Rectangle(URectangle::new(
                dim_text.width + 10.0,
                dim_text.height,
            )));
        let line_pos = self.h1(sb);
        draw_hline(ug, line_pos, 0.0, Self::DELTAX);
        draw_hline(ug, line_pos, dim.width - Self::DELTAX, dim.width);
        self.orig
            .draw_u(&ug.translated(Self::DELTAX, Self::DELTAY1));
        draw_text(ug, &self.group_name, &self.font, 5.0, 0.0);
    }
}

/// The widest of the texts, and their heights added.
fn list_dimension(
    elements: &[String],
    font: &FontConfiguration,
    sb: &dyn StringBounder,
) -> XDimension2D {
    let mut width: f64 = 0.0;
    let mut height = 0.0;
    for element in elements {
        let dim = text_dimension(font, element, sb);
        width = width.max(dim.width);
        height += dim.height;
    }
    XDimension2D::new(width, height)
}

/// The texts one below the other, `x` from the left, separated by thin lines `separator_width` long.
fn draw_list(
    ug: &UGraphic,
    elements: &[String],
    font: &FontConfiguration,
    x: f64,
    separator_width: f64,
) {
    let sb = ug.string_bounder();
    let mut y = 0.0;
    for element in elements {
        let dim_element = text_dimension(font, element, sb);
        draw_text(
            ug,
            element,
            font,
            x,
            y + dim_element.height - descent(font, element, sb),
        );
        if y > 0.0 {
            draw_hline(
                &ug.with_stroke(UStroke::with_thickness(0.3)),
                y,
                0.0,
                separator_width,
            );
        }
        y += dim_element.height;
    }
}

const DASHED: UStroke = UStroke {
    dash_visible: 5.0,
    dash_space: 5.0,
    thickness: 1.0,
};

/// The characters of a class `[…]`, one below the other in a dashed box (`ETileRegexGroup`).
pub(crate) struct ETileRegexGroup {
    elements: Vec<String>,
    style: TileStyle,
}

impl ETileRegexGroup {
    pub(crate) fn new(elements: Vec<String>, style: TileStyle) -> Self {
        Self { elements, style }
    }

    fn text_dim(&self, sb: &dyn StringBounder) -> XDimension2D {
        list_dimension(&self.elements, &self.style.font, sb)
    }

    fn half_height(&self, sb: &dyn StringBounder) -> f64 {
        self.text_dim(sb).height / 2.0
    }

    fn width(&self, sb: &dyn StringBounder) -> f64 {
        self.text_dim(sb).width + 10.0
    }

    /// The box is as wide as the tile, so no track leads to it.
    fn draw_u(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let dim_box = self.text_dim(sb).delta(10.0, 0.0);
        ug.with_color(self.style.line_color.clone())
            .with_stroke(DASHED)
            .draw(&UShape::Rectangle(URectangle::new(
                dim_box.width,
                dim_box.height,
            )));
        draw_list(ug, &self.elements, &self.style.font, 5.0, dim_box.width);
    }
}

/// A negated class `[^…]`: the `^` in a bracket, then the characters it excludes (`ETileRegexGroupAllBut`).
pub(crate) struct ETileRegexGroupAllBut {
    elements1: Vec<String>,
    elements2: Vec<String>,
    style: TileStyle,
}

impl ETileRegexGroupAllBut {
    pub(crate) fn new(mut elements: Vec<String>, style: TileStyle) -> Self {
        let elements2 = elements.split_off(1);
        Self {
            elements1: elements,
            elements2,
            style,
        }
    }

    fn half_height(&self, sb: &dyn StringBounder) -> f64 {
        let height1 = list_dimension(&self.elements1, &self.style.font, sb).height;
        let height2 = list_dimension(&self.elements2, &self.style.font, sb).height;
        height1.max(height2) / 2.0
    }

    fn width(&self, sb: &dyn StringBounder) -> f64 {
        list_dimension(&self.elements1, &self.style.font, sb).width
            + list_dimension(&self.elements2, &self.style.font, sb).width
            + 20.0
    }

    fn draw_u(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let font = &self.style.font;
        let total_height = 2.0 * self.half_height(sb);
        let dim1 = list_dimension(&self.elements1, font, sb);
        let dim2 = list_dimension(&self.elements2, font, sb);
        let box1 = dim1.delta(10.0, 0.0);
        let box2 = dim2.delta(10.0, 0.0);
        let ug_box = ug
            .with_color(self.style.line_color.clone())
            .with_stroke(DASHED);
        ug_box
            .translated(box1.width, 0.0)
            .draw(&UShape::Rectangle(URectangle::new(box2.width, box2.height)));
        ug_box
            .translated(0.0, (total_height - box1.height) / 2.0)
            .draw(&UShape::path(vec![
                USegment::MoveTo(box1.width, 0.0),
                USegment::LineTo(0.0, 0.0),
                USegment::LineTo(0.0, box1.height),
                USegment::LineTo(box1.width, box1.height),
            ]));
        let separator_width = 10.0 + dim1.width;
        draw_list(
            &ug.translated(0.0, (total_height - dim1.height) / 2.0),
            &self.elements1,
            font,
            5.0,
            separator_width,
        );
        draw_list(
            &ug.translated(10.0 + dim1.width, 0.0),
            &self.elements2,
            font,
            5.0,
            separator_width,
        );
    }
}
