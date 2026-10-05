//! The widgets of a salt mock-up and the grid that holds them.

use std::cell::OnceCell;
use std::sync::LazyLock;

use regex::Regex;

use super::data_source::Terminator;
use crate::color::HColor;
use crate::creole::{CreoleParser, Display, SheetBlock1};
use crate::java::{self, JavaHashSet};
use crate::klimt::font::{FontConfiguration, StringBounder, UFont};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::shape::{UEllipse, URectangle, USegment, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::pattern::java_regex;

/// Salt draws in two passes (`z_index` 0 then 1), so that open drop-lists cover later widgets.
pub(super) trait Element {
    fn preferred_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D;

    fn draw_u(&self, ug: &UGraphic, z_index: i32, dimension: XDimension2D);

    /// Plain text cells can mean something to the grid around them, like `*` merging cells.
    fn plain_text(&self) -> Option<&str> {
        None
    }
}

/// Every widget's text is `SansSerif` 12.
pub(super) fn widget_font() -> UFont {
    UFont::new("SansSerif", crate::klimt::font::UFontFace::NORMAL, 12)
}

pub(super) fn color(name: &str) -> HColor {
    HColor::parse(name).ok().flatten().unwrap_or(HColor::WHITE)
}

pub(super) fn text_block(lines: &[String], font: &FontConfiguration) -> SheetBlock1 {
    let sheet = CreoleParser::new(font.clone(), HorizontalAlignment::Left).create_sheet(lines);
    SheetBlock1::new(sheet, ClockwiseTopRightBottomLeft::none())
}

pub(super) struct Text {
    block: SheetBlock1,
    text: String,
}

impl Text {
    pub(super) fn new(text: &str, font: UFont) -> Self {
        Self {
            block: text_block(
                &[text.to_owned()],
                &FontConfiguration::black_blue_true(font),
            ),
            text: text.to_owned(),
        }
    }
}

impl Element for Text {
    fn preferred_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.block.calculate_dimension(string_bounder)
    }

    /// A lone `.` keeps its cell but shows nothing.
    fn draw_u(&self, ug: &UGraphic, z_index: i32, _dimension: XDimension2D) {
        if z_index == 0 && self.text != "." {
            self.block.draw_u(ug);
        }
    }

    fn plain_text(&self) -> Option<&str> {
        Some(&self.text)
    }
}

/// Text in a widget that is at least as wide as its characters, spaces included, at 8 points each.
struct WidgetText {
    block: SheetBlock1,
    char_length: usize,
}

impl WidgetText {
    fn new(text: &str, font: UFont) -> Self {
        static ICON: LazyLock<Regex> = LazyLock::new(|| java_regex(r"<&[-\w]+>", false));
        let char_length = purge_all_tags(&ICON.replace_all(text, "00"))
            .encode_utf16()
            .count();
        let font = FontConfiguration::black_blue_true(font);
        Self {
            block: text_block(&[java::trim(text).to_owned()], &font),
            char_length,
        }
    }

    fn pure_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.block.calculate_dimension(string_bounder)
    }

    fn dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let pure = self.pure_dimension(string_bounder);
        XDimension2D::new(pure.width.max(self.char_length as f64 * 8.0), pure.height)
    }

    fn draw(&self, ug: &UGraphic, x: f64, y: f64) {
        self.block.draw_u(&ug.translated(x, y));
    }
}

/// `Splitter.purgeAllTag`: removes creole tags. PlantUML compiles this pattern without expanding its
/// `%s`-style shorthands, which then stand for themselves.
fn purge_all_tags(text: &str) -> String {
    static TAG: LazyLock<Regex> = LazyLock::new(|| {
        let font = r"\<font(\s+size[%s]*=[%s]*[%g]?\d+[%g]?|[%s]+color[%s]*=\s*[%g]?(#[0-9a-fA-F]{1,6}|\w+)[%g]?)+[%s]*\>";
        let parts = [
            r"\<[pP][lL][aA][iI][nN]\>",
            r"\</[pP][lL][aA][iI][nN]\>",
            r"\<[iI]\>",
            r"\</[iI]\>",
            r"\<[bB]\>",
            r"\</[bB]\>",
            r"\<[uU](?::(#[0-9a-fA-F]{6}|\w+))?\>",
            r"\</[uU]\>",
            "<【strike┇STRIKE┇s┇S┇del┇DEL】〇?〘:〶$XC=【#〇{6}「0〜9a〜fA〜F」┇〇+〴w】〙>",
            r"\</(?:s|S|strike|STRIKE|del|DEL)\>",
            r"\<[wW](?::(#[0-9a-fA-F]{6}|\w+))?\>",
            r"\</[wW]\>",
            r"\<[bB][aA][cC][kK](?::(#?\w+(?:[-\\|/]#?\w+)?))?\>",
            r"\</[bB][aA][cC][kK]\>",
            font,
            r"\<color[\s:]+(#[0-9a-fA-F]{1,6}|#?\w+)[%s]*\>",
            r"\<size[\s:]+(\d+)[%s]*\>",
            r"\<sup\>",
            r"\<sub\>",
            r"\</font\>|\</color\>|\</size\>|\</text\>",
            r"\</sup\>|\</sub\>",
            r"\<qrcode[\s:]+([^>{}]+)(\{scale=(?:[0-9.]+)\})?\>",
            r"\<img\s+(src[%s]*=[%s]*[%q%g]?[^\s%g>]+[%q%g]?[%s]*|vspace\s*=\s*[%q%g]?\d+[%q%g]?\s*|valign[%s]*=[%s]*[%q%g]?(top|middle|bottom)[%q%g]?[%s]*)+\>",
            r"\<img[\s:]+([^>{}]+)(\{scale=(?:[0-9.]+)\})?\>",
            r"\<font[\s:]+([^>]+)/?\>",
            r"\[\[([^\[\]]+)\]\]",
            r"\<text[\s:]+([^>]+)/?\>",
        ];
        java_regex(&parts.join("|"), false)
    });
    TAG.replace_all(text, "").into_owned()
}

pub(super) struct Button {
    text: WidgetText,
}

impl Button {
    const STROKE: f64 = 2.5;
    const MARGIN: f64 = 2.0;

    pub(super) fn new(text: &str, font: UFont) -> Self {
        Self {
            text: WidgetText::new(text, font),
        }
    }
}

impl Element for Button {
    fn preferred_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let stroke = Self::STROKE;
        self.text
            .dimension(string_bounder)
            .delta(2.0 * Self::MARGIN, 2.0 * Self::MARGIN)
            .delta(2.0 * stroke, 2.0 * stroke)
    }

    fn draw_u(&self, ug: &UGraphic, z_index: i32, _dimension: XDimension2D) {
        if z_index != 0 {
            return;
        }
        let string_bounder = ug.string_bounder();
        let dimension = self.preferred_dimension(string_bounder);
        let stroke = Self::STROKE;
        let ug = ug
            .with_stroke(UStroke::with_thickness(stroke))
            .with_backcolor(color("#E"))
            .with_color(HColor::BLACK);
        let frame = URectangle::new(
            dimension.width - 2.0 * stroke,
            dimension.height - 2.0 * stroke,
        )
        .rounded(10.0);
        ug.translated(stroke, stroke)
            .draw(&UShape::Rectangle(frame));
        let text_width = self.text.pure_dimension(string_bounder).width;
        self.text.draw(
            &ug,
            (dimension.width - text_width) / 2.0,
            stroke + Self::MARGIN,
        );
    }
}

pub(super) struct TextField {
    text: WidgetText,
}

impl TextField {
    pub(super) fn new(text: &str, font: UFont) -> Self {
        Self {
            text: WidgetText::new(text, font),
        }
    }
}

impl Element for TextField {
    fn preferred_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.text.dimension(string_bounder).delta(6.0, 2.0)
    }

    /// The text over a line with short ticks at both ends.
    fn draw_u(&self, ug: &UGraphic, z_index: i32, _dimension: XDimension2D) {
        if z_index != 0 {
            return;
        }
        let string_bounder = ug.string_bounder();
        self.text.draw(ug, 3.0, 0.0);
        let width = self.preferred_dimension(string_bounder).width;
        let text = self.text.dimension(string_bounder);
        ug.translated(1.0, text.height).draw(&UShape::Line {
            dx: width - 3.0,
            dy: 0.0,
        });
        let tick_top = text.height - 3.0;
        let tick = UShape::Line { dx: 0.0, dy: 2.0 };
        ug.translated(1.0, tick_top).draw(&tick);
        ug.translated(3.0 + text.width + 1.0, tick_top).draw(&tick);
    }
}

pub(super) struct RadioCheckbox {
    block: SheetBlock1,
    radio: bool,
    checked: bool,
}

impl RadioCheckbox {
    const MARGIN: f64 = 20.0;
    const BOX: f64 = 10.0;
    const DOT: f64 = 4.0;

    pub(super) fn checkbox_on(text: &str, font: UFont) -> Self {
        Self::new(text, font, false, true)
    }

    pub(super) fn checkbox_off(text: &str, font: UFont) -> Self {
        Self::new(text, font, false, false)
    }

    pub(super) fn radio_on(text: &str, font: UFont) -> Self {
        Self::new(text, font, true, true)
    }

    pub(super) fn radio_off(text: &str, font: UFont) -> Self {
        Self::new(text, font, true, false)
    }

    fn new(text: &str, font: UFont, radio: bool, checked: bool) -> Self {
        Self {
            block: text_block(
                &[text.to_owned()],
                &FontConfiguration::black_blue_true(font),
            ),
            radio,
            checked,
        }
    }
}

impl Element for RadioCheckbox {
    fn preferred_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.block
            .calculate_dimension(string_bounder)
            .delta(Self::MARGIN, 0.0)
    }

    fn draw_u(&self, ug: &UGraphic, z_index: i32, _dimension: XDimension2D) {
        if z_index != 0 {
            return;
        }
        let ug = ug.with_color(HColor::BLACK);
        self.block.draw_u(&ug.translated(Self::MARGIN, 0.0));
        let height = self.preferred_dimension(ug.string_bounder()).height;
        let ug = ug.with_stroke(UStroke::with_thickness(1.5));
        let mark = ug.with_backcolor(ug.param().color.clone());
        if self.radio {
            let circle = UShape::Ellipse(UEllipse::new(Self::BOX, Self::BOX));
            ug.translated(2.0, (height - Self::BOX) / 2.0).draw(&circle);
            if self.checked {
                let x = 2.0 + f64::from(((Self::BOX - Self::DOT) / 2.0) as i32);
                mark.translated(x, (height - Self::DOT) / 2.0)
                    .draw(&UShape::Ellipse(UEllipse::new(Self::DOT, Self::DOT)));
            }
        } else {
            let square = UShape::Rectangle(URectangle::new(Self::BOX, Self::BOX));
            ug.translated(2.0, (height - Self::BOX) / 2.0).draw(&square);
            if self.checked {
                let tick = vec![(0.0, 0.0), (3.0, 3.0), (10.0, -6.0), (3.0, 1.0)];
                mark.translated(3.0, 6.0).draw(&UShape::Polygon(tick));
            }
        }
    }
}

pub(super) struct Droplist {
    text: WidgetText,
    open: Option<SheetBlock1>,
}

impl Droplist {
    const BOX: f64 = 12.0;

    /// `shown^option^option`: the shown value, then the options of an open list.
    pub(super) fn new(text: &str, font: UFont) -> Self {
        let shown = text.find('^').map_or(text, |index| &text[..index]);
        let options: Vec<String> = text
            .split('^')
            .filter(|option| !option.is_empty())
            .skip(1)
            .map(str::to_owned)
            .collect();
        let font_configuration = FontConfiguration::black_blue_true(font.clone());
        Self {
            text: WidgetText::new(shown, font),
            open: (!options.is_empty()).then(|| text_block(&options, &font_configuration)),
        }
    }
}

impl Element for Droplist {
    fn preferred_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.text
            .dimension(string_bounder)
            .delta(4.0 + Self::BOX, 4.0)
    }

    fn draw_u(&self, ug: &UGraphic, z_index: i32, _dimension: XDimension2D) {
        let string_bounder = ug.string_bounder();
        let dimension = self.preferred_dimension(string_bounder);
        let ug = ug.with_color(HColor::BLACK);
        if z_index == 0 {
            let frame = URectangle::new(dimension.width - 1.0, dimension.height - 1.0);
            ug.with_backcolor(color("#E"))
                .draw(&UShape::Rectangle(frame));
            self.text.draw(&ug, 2.0, 2.0);
            let x_line = dimension.width - Self::BOX;
            ug.translated(x_line, 0.0).draw(&UShape::Line {
                dx: 0.0,
                dy: dimension.height - 1.0,
            });
            let text_height = self.text.pure_dimension(string_bounder).height;
            let arrow = vec![
                (0.0, 0.0),
                (Self::BOX - 6.0, 0.0),
                (
                    f64::from(((Self::BOX - 6.0) / 2.0) as i32),
                    text_height - 8.0,
                ),
            ];
            ug.with_backcolor(ug.param().color.clone())
                .translated(x_line + 3.0, 6.0)
                .draw(&UShape::Polygon(arrow));
        }
        if let Some(open) = &self.open {
            let options = open.calculate_dimension(string_bounder);
            let width = options.width.max(dimension.width - 1.0);
            let ug = ug.translated(0.0, dimension.height - 1.0);
            let frame = URectangle::new(width - 1.0, options.height - 1.0);
            ug.with_backcolor(color("#E"))
                .draw(&UShape::Rectangle(frame));
            open.draw_u(&ug);
        }
    }
}

/// A separator across its cell: `--` plain, `==` double, `..` dotted, `~~` thick.
pub(super) struct Line {
    separator: char,
}

impl Line {
    pub(super) fn new(separator: char) -> Self {
        Self { separator }
    }
}

impl Element for Line {
    fn preferred_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(10.0, 6.0)
    }

    fn draw_u(&self, ug: &UGraphic, z_index: i32, dimension: XDimension2D) {
        if z_index != 0 {
            return;
        }
        let ug = ug.with_color(color("#A"));
        let middle = dimension.height / 2.0;
        let line = UShape::Line {
            dx: dimension.width,
            dy: 0.0,
        };
        let draw_at =
            |y: f64, stroke: UStroke| ug.with_stroke(stroke).translated(0.0, y).draw(&line);
        match self.separator {
            '=' => {
                let top = middle - 1.0;
                draw_at(top, UStroke::SIMPLE);
                draw_at(top + 2.0, UStroke::SIMPLE);
            }
            '.' => draw_at(
                middle,
                UStroke {
                    dash_visible: 1.0,
                    dash_space: 2.0,
                    thickness: 1.0,
                },
            ),
            '-' => draw_at(middle, UStroke::SIMPLE),
            _ => draw_at(middle, UStroke::with_thickness(1.5)),
        }
    }
}

/// Which lines of a grid are drawn: `{` none, `{+` around, `{^` around with a title, `{-` between rows,
/// `{!` between columns, `{#` all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TableStrategy {
    None,
    Outside,
    OutsideWithTitle,
    Horizontal,
    Vertical,
    All,
}

impl TableStrategy {
    pub(super) fn from_char(c: char) -> Option<Self> {
        match c {
            ' ' => Some(Self::None),
            '+' => Some(Self::Outside),
            '^' => Some(Self::OutsideWithTitle),
            '-' => Some(Self::Horizontal),
            '!' => Some(Self::Vertical),
            '#' => Some(Self::All),
            _ => None,
        }
    }
}

/// The rows and columns an element spans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cell {
    min_row: usize,
    max_row: usize,
    min_col: usize,
    max_col: usize,
}

impl Cell {
    fn columns(&self) -> usize {
        self.max_col - self.min_col + 1
    }

    fn rows(&self) -> usize {
        self.max_row - self.min_row + 1
    }
}

/// Places elements in rows and columns as their terminators say (PlantUML's `Positionner2`).
#[derive(Default)]
pub(super) struct Positionner {
    row: usize,
    col: usize,
    max_row: usize,
    max_col: usize,
    cells: Vec<(Box<dyn Element>, Cell)>,
}

impl Positionner {
    pub(super) fn add(&mut self, element: Box<dyn Element>, terminator: Terminator) {
        let cell = Cell {
            min_row: self.row,
            max_row: self.row,
            min_col: self.col,
            max_col: self.col,
        };
        self.cells.push((element, cell));
        self.update_max();
        self.advance(terminator);
    }

    /// `*`: the previous element spans one more column.
    pub(super) fn merge_left(&mut self, terminator: Terminator) {
        self.update_max();
        self.advance(terminator);
        if let Some((_, last)) = self.cells.last_mut() {
            last.max_col += 1;
        }
    }

    fn advance(&mut self, terminator: Terminator) {
        match terminator {
            Terminator::NewColumn => self.col += 1,
            Terminator::NewLine => {
                self.row += 1;
                self.col = 0;
            }
        }
    }

    fn update_max(&mut self) {
        self.max_row = self.max_row.max(self.row);
        self.max_col = self.max_col.max(self.col);
    }
}

/// A grid of elements (`{ ... }`), sized so that every element fits its cells.
pub(super) struct Pyramid {
    cells: Vec<(Box<dyn Element>, Cell)>,
    rows: usize,
    cols: usize,
    strategy: TableStrategy,
    title: Option<SheetBlock1>,
    /// Computed on the first measurement, like `ElementPyramid.init`: re-measuring nested grids on every
    /// call is exponential in their depth. Each export builds its own elements and measures them with
    /// one string bounder.
    starts: OnceCell<Starts>,
}

/// Where each row and column starts, the last entry being where the grid ends.
struct Starts {
    rows: Vec<f64>,
    cols: Vec<f64>,
}

impl Pyramid {
    /// Enough rows and columns for every cell. PlantUML counts the cells' last row and column instead
    /// of one past them, and so crashes when a row starting with `*` widens the cell above it beyond
    /// the widest row.
    pub(super) fn new(
        positionner: Positionner,
        strategy: TableStrategy,
        title: Option<&str>,
    ) -> Self {
        let rows = positionner
            .cells
            .iter()
            .map(|(_, cell)| cell.max_row + 1)
            .fold(positionner.max_row + 1, usize::max);
        let cols = positionner
            .cells
            .iter()
            .map(|(_, cell)| cell.max_col + 1)
            .fold(positionner.max_col + 1, usize::max);
        let title = title.map(|title| {
            let font = FontConfiguration::black_blue_true(widget_font());
            text_block(Display::with_newlines(title).lines(), &font)
        });
        Self {
            cells: positionner.cells,
            rows,
            cols,
            strategy,
            title,
            starts: OnceCell::new(),
        }
    }

    fn title_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.title.as_ref().map_or(0.0, |title| {
            title.calculate_dimension(string_bounder).height
        })
    }

    fn starts(&self, string_bounder: &dyn StringBounder) -> &Starts {
        self.starts
            .get_or_init(|| self.compute_starts(string_bounder))
    }

    fn compute_starts(&self, string_bounder: &dyn StringBounder) -> Starts {
        let title_height = self.title_height(string_bounder);
        let mut rows_start = vec![title_height / 2.0; self.rows + 1];
        let mut cols_start = vec![0.0; self.cols + 1];
        let mut order: Vec<usize> = (0..self.cells.len()).collect();
        order.sort_by_key(|&index| {
            let cell = self.cells[index].1;
            (cell.columns(), cell.min_col)
        });
        for &index in &order {
            let (element, cell) = &self.cells[index];
            let width = element.preferred_dimension(string_bounder).width + 2.0;
            ensure_span(&mut cols_start, cell.min_col, cell.max_col + 1, width);
        }
        order.sort_by_key(|&index| {
            let cell = self.cells[index].1;
            (cell.rows(), cell.min_row)
        });
        for &index in &order {
            let (element, cell) = &self.cells[index];
            let above = if cell.min_row == 0 {
                title_height / 2.0
            } else {
                0.0
            };
            let height = element.preferred_dimension(string_bounder).height + above + 2.0;
            ensure_span(&mut rows_start, cell.min_row, cell.max_row + 1, height);
        }
        Starts {
            rows: rows_start,
            cols: cols_start,
        }
    }
}

/// Pushes the starts from `last` on so that the span from `first` to `last` is at least `size`.
fn ensure_span(starts: &mut [f64], first: usize, last: usize, size: f64) {
    let missing = size - (starts[last] - starts[first]);
    if missing > 0.0 {
        for start in &mut starts[last..] {
            *start += missing;
        }
    }
}

impl Element for Pyramid {
    fn preferred_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let Starts { rows, cols } = self.starts(string_bounder);
        XDimension2D::new(
            cols[cols.len() - 1],
            rows[rows.len() - 1] + self.title_height(string_bounder),
        )
    }

    fn draw_u(&self, ug: &UGraphic, z_index: i32, _dimension: XDimension2D) {
        let ug = ug.with_color(HColor::BLACK);
        let string_bounder = ug.string_bounder();
        let title_height = self.title_height(string_bounder);
        let Starts {
            rows: rows_start,
            cols: cols_start,
        } = self.starts(string_bounder);
        let mut grid = Grid::new(rows_start, cols_start, self.strategy);
        for (element, cell) in &self.cells {
            let above = if cell.min_row == 0 {
                title_height / 2.0
            } else {
                0.0
            };
            let x = cols_start[cell.min_col];
            let y = rows_start[cell.min_row] + above;
            let width = cols_start[cell.max_col + 1] - x - 1.0;
            let height = rows_start[cell.max_row + 1] - rows_start[cell.min_row] - 1.0;
            grid.add_cell(cell);
            element.draw_u(
                &ug.translated(x + 1.0, y + 1.0),
                z_index,
                XDimension2D::new(width, height),
            );
        }
        if z_index == 0 {
            grid.draw_u(&ug, self.title.as_ref());
        }
    }
}

/// Which scroll bars a scroll pane shows: `{S` both, `{SI` vertical, `{S-` horizontal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ScrollStrategy {
    Both,
    VerticalOnly,
    HorizontalOnly,
}

impl ScrollStrategy {
    pub(super) fn from_desc(header: &str) -> Self {
        if header.ends_with('-') {
            Self::HorizontalOnly
        } else if header.ends_with('I') {
            Self::VerticalOnly
        } else {
            Self::Both
        }
    }
}

/// A framed grid with scroll bars along it.
pub(super) struct PyramidScrolled {
    pyramid: Pyramid,
    scroll_strategy: ScrollStrategy,
}

impl PyramidScrolled {
    const BAR_THICKNESS: f64 = 15.0;
    const ARROW_BOX_LENGTH: f64 = 12.0;

    pub(super) fn new(positionner: Positionner, scroll_strategy: ScrollStrategy) -> Self {
        Self {
            pyramid: Pyramid::new(positionner, TableStrategy::Outside, None),
            scroll_strategy,
        }
    }

    fn draw_vertical(ug: &UGraphic, width: f64, height: f64) {
        ug.draw(&UShape::Rectangle(URectangle::new(width, height)));
        let hline = UShape::Line { dx: width, dy: 0.0 };
        ug.translated(0.0, Self::ARROW_BOX_LENGTH).draw(&hline);
        ug.translated(0.0, height - Self::ARROW_BOX_LENGTH)
            .draw(&hline);
        let arrows = ug.with_backcolor(HColor::BLACK);
        arrows
            .translated(4.0, 4.0)
            .draw(&triangle([(3.0, 0.0), (6.0, 5.0), (0.0, 5.0)]));
        arrows
            .translated(4.0, height - Self::ARROW_BOX_LENGTH + 4.0)
            .draw(&triangle([(3.0, 5.0), (6.0, 0.0), (0.0, 0.0)]));
    }

    fn draw_horizontal(ug: &UGraphic, width: f64, height: f64) {
        ug.draw(&UShape::Rectangle(URectangle::new(width, height)));
        let vline = UShape::Line {
            dx: 0.0,
            dy: height,
        };
        ug.translated(Self::ARROW_BOX_LENGTH, 0.0).draw(&vline);
        ug.translated(width - Self::ARROW_BOX_LENGTH, 0.0)
            .draw(&vline);
        let arrows = ug.with_backcolor(HColor::BLACK);
        arrows
            .translated(4.0, 4.0)
            .draw(&triangle([(0.0, 3.0), (5.0, 6.0), (5.0, 0.0)]));
        arrows
            .translated(width - Self::ARROW_BOX_LENGTH + 4.0, 4.0)
            .draw(&triangle([(5.0, 3.0), (0.0, 6.0), (0.0, 0.0)]));
    }
}

/// A closed path through three corners, drawn as PlantUML draws it.
fn triangle(corners: [(f64, f64); 3]) -> UShape {
    let [(x, y), second, third] = corners;
    UShape::Path(vec![
        USegment::MoveTo(x, y),
        USegment::LineTo(second.0, second.1),
        USegment::LineTo(third.0, third.1),
        USegment::LineTo(x, y),
    ])
}

impl Element for PyramidScrolled {
    fn preferred_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let pyramid = self.pyramid.preferred_dimension(string_bounder);
        match self.scroll_strategy {
            ScrollStrategy::HorizontalOnly => pyramid.delta(0.0, 30.0),
            ScrollStrategy::VerticalOnly => pyramid.delta(30.0, 0.0),
            ScrollStrategy::Both => pyramid.delta(30.0, 30.0),
        }
    }

    /// The bars are drawn in both passes.
    fn draw_u(&self, ug: &UGraphic, z_index: i32, dimension: XDimension2D) {
        self.pyramid.draw_u(ug, z_index, dimension);
        let ug = ug.with_color(HColor::BLACK);
        let pyramid = self.pyramid.preferred_dimension(ug.string_bounder());
        if self.scroll_strategy != ScrollStrategy::HorizontalOnly {
            Self::draw_vertical(
                &ug.translated(pyramid.width + 4.0, 0.0),
                Self::BAR_THICKNESS,
                pyramid.height,
            );
        }
        if self.scroll_strategy != ScrollStrategy::VerticalOnly {
            Self::draw_horizontal(
                &ug.translated(0.0, pyramid.height + 4.0),
                pyramid.width,
                Self::BAR_THICKNESS,
            );
        }
    }
}

/// The lines of a grid, one segment per cell side. PlantUML keeps them in hash sets, whose order is the
/// drawing order.
struct Grid<'a> {
    rows_start: &'a [f64],
    cols_start: &'a [f64],
    strategy: TableStrategy,
    horizontals: JavaHashSet<(usize, usize)>,
    verticals: JavaHashSet<(usize, usize)>,
}

impl<'a> Grid<'a> {
    fn new(rows_start: &'a [f64], cols_start: &'a [f64], strategy: TableStrategy) -> Self {
        let mut grid = Self {
            rows_start,
            cols_start,
            strategy,
            horizontals: JavaHashSet::default(),
            verticals: JavaHashSet::default(),
        };
        if matches!(
            strategy,
            TableStrategy::Outside | TableStrategy::OutsideWithTitle | TableStrategy::All
        ) {
            let (rows, cols) = (rows_start.len(), cols_start.len());
            for col in 0..cols - 1 {
                grid.horizontal(0, col);
                grid.horizontal(rows - 1, col);
            }
            for row in 0..rows - 1 {
                grid.vertical(row, 0);
                grid.vertical(row, cols - 1);
            }
        }
        grid
    }

    fn horizontal(&mut self, row: usize, col: usize) {
        self.horizontals.insert((row, col), segment_hash(row, col));
    }

    fn vertical(&mut self, row: usize, col: usize) {
        self.verticals.insert((row, col), segment_hash(row, col));
    }

    fn add_cell(&mut self, cell: &Cell) {
        if matches!(
            self.strategy,
            TableStrategy::Horizontal | TableStrategy::All
        ) {
            for col in cell.min_col..=cell.max_col {
                self.horizontal(cell.min_row, col);
                self.horizontal(cell.max_row + 1, col);
            }
        }
        if matches!(self.strategy, TableStrategy::Vertical | TableStrategy::All) {
            for row in cell.min_row..=cell.max_row {
                self.vertical(row, cell.min_col);
                self.vertical(row, cell.max_col + 1);
            }
        }
    }

    fn draw_u(&self, ug: &UGraphic, title: Option<&SheetBlock1>) {
        for &(row, col) in self.horizontals.iter() {
            let width = self.cols_start[col + 1] - self.cols_start[col];
            ug.translated(self.cols_start[col], self.rows_start[row])
                .draw(&UShape::Line { dx: width, dy: 0.0 });
        }
        for &(row, col) in self.verticals.iter() {
            let height = self.rows_start[row + 1] - self.rows_start[row];
            ug.translated(self.cols_start[col], self.rows_start[row])
                .draw(&UShape::Line {
                    dx: 0.0,
                    dy: height,
                });
        }
        if let Some(title) = title {
            let dimension = title.calculate_dimension(ug.string_bounder());
            if dimension.width > 0.0 && dimension.height > 0.0 {
                let ug = ug.translated(6.0, 0.0);
                ug.with_backcolor(HColor::WHITE)
                    .with_color(HColor::WHITE)
                    .draw(&UShape::Rectangle(URectangle::new(
                        dimension.width,
                        dimension.height,
                    )));
                title.draw_u(&ug);
            }
        }
    }
}

/// `Segment.hashCode`.
fn segment_hash(row: usize, col: usize) -> i32 {
    (row * 47 + col) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_do_not_count_as_characters() {
        assert_eq!(purge_all_tags("<b>bold</b> <color:red>x</color>"), "bold x");
        assert_eq!(purge_all_tags("[[http://a]] <size:9>y"), " y");
    }
}
