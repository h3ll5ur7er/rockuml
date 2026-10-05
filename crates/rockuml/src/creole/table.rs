//! Creole tables: `|= header |= header |` and `| cell | cell |` lines, optionally coloured with `<#back,#line>`
//! (PlantUML's `StripeTable` and `AtomTable`).

use std::sync::LazyLock;

use regex::Regex;

use super::parser::StripeBuilder;
use super::sheet_block::SheetBlock1;
use super::{Atom, CreoleMode, Sheet};
use crate::color::HColor;
use crate::jaws::BLOCK_E1_NEWLINE;
use crate::klimt::font::{FontConfiguration, FontStyle, StringBounder};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::pattern::java_regex;

const HIDDEN_BAR: char = '\u{E000}';

pub(super) fn is_table_line(line: &str) -> bool {
    static TABLE_LINE: LazyLock<Regex> =
        LazyLock::new(|| java_regex(r"^(\<#\w+(,#?\w+)?\>)?\|(\=)?.*\|$", false));
    TABLE_LINE.is_match(line)
}

pub(super) struct AtomTable {
    lines: Vec<Line>,
    line_color: HColor,
}

struct Line {
    back_color: Option<HColor>,
    cells: Vec<Cell>,
}

struct Cell {
    content: SheetBlock1,
    back_color: Option<HColor>,
}

impl AtomTable {
    /// A table started by `line`, whose colour prefix may also give the colour of the grid.
    pub(super) fn new(line: &str, font: &FontConfiguration) -> Self {
        let line_color = leading_color(line, 1).unwrap_or_else(|| font.color().clone());
        let mut table = Self {
            lines: Vec::new(),
            line_color,
        };
        table.add_line(line, font);
        table
    }

    pub(super) fn add_line(&mut self, line: &str, font: &FontConfiguration) {
        let line = line.replace("\\|", &HIDDEN_BAR.to_string());
        let back_color = leading_color(&line, 0);
        let line = if back_color.is_some() {
            without_leading_color(&line)
        } else {
            &line
        };
        let cells = line
            .split('|')
            .filter(|token| !token.is_empty())
            .map(|token| cell(&token.replace(HIDDEN_BAR, "|"), font))
            .collect();
        self.lines.push(Line { back_color, cells });
    }

    fn column_count(&self) -> usize {
        self.lines
            .iter()
            .map(|line| line.cells.len())
            .max()
            .unwrap_or(0)
    }

    /// Each column as wide as its widest cell, each line as high as its highest.
    fn grid(&self, string_bounder: &dyn StringBounder) -> (Vec<f64>, Vec<f64>) {
        let dimensions: Vec<Vec<XDimension2D>> = self
            .lines
            .iter()
            .map(|line| {
                line.cells
                    .iter()
                    .map(|cell| cell.content.calculate_dimension(string_bounder))
                    .collect()
            })
            .collect();
        let widths = (0..self.column_count())
            .map(|column| {
                dimensions
                    .iter()
                    .filter_map(|line| line.get(column))
                    .map(|dimension| dimension.width)
                    .fold(0.0, f64::max)
            })
            .collect();
        let heights = dimensions
            .iter()
            .map(|line| {
                line.iter()
                    .map(|dimension| dimension.height)
                    .fold(0.0, f64::max)
            })
            .collect();
        (widths, heights)
    }
}

/// The colour at `index` in a leading `<#back,#line>`; `#` is optional after the first.
fn leading_color(text: &str, index: usize) -> Option<HColor> {
    static STARTS_WITH_COLOR: LazyLock<Regex> =
        LazyLock::new(|| java_regex(r"^\=?\s*(\<#\w+(,#?\w+)?\>).*", false));
    if !STARTS_WITH_COLOR.is_match(text) {
        return None;
    }
    let start = text.find('#')?;
    let end = text.find('>')?;
    let name = text[start..end].split(',').nth(index)?;
    Some(HColor::parse_or_white(name))
}

fn without_leading_color(text: &str) -> &str {
    &text[text.find('>').map_or(0, |end| end + 1)..]
}

/// A `=` cell is a header, in bold; `\n` breaks the cell into lines.
fn cell(token: &str, font: &FontConfiguration) -> Cell {
    let (token, font) = match token.strip_prefix('=') {
        Some(header) => (header, font.with_style(FontStyle::Bold)),
        None => (token, font.clone()),
    };
    let back_color = leading_color(token, 0);
    let token = if back_color.is_some() {
        without_leading_color(token)
    } else {
        token
    };
    let stripes = cell_lines(token)
        .iter()
        .map(|text| {
            let mut stripe = StripeBuilder::plain(font.clone(), CreoleMode::Full);
            stripe.analyze_and_add(text);
            stripe.build()
        })
        .collect();
    Cell {
        content: SheetBlock1::new(Sheet { stripes }, ClockwiseTopRightBottomLeft::none()),
        back_color,
    }
}

/// The lines of a table cell or tree item: broken at a written `\n` or a hidden newline, with `\\` written as a
/// single backslash (PlantUML's `getWithNewlinesInternal`).
pub(super) fn cell_lines(text: &str) -> Vec<String> {
    let mut lines = vec![String::new()];
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        let current = lines.last_mut().expect("there is always a current line");
        match c {
            '\\' if chars.peek().is_some() => match chars.next().expect("peeked") {
                'n' => lines.push(String::new()),
                '\\' => current.push('\\'),
                other => {
                    current.push('\\');
                    current.push(other);
                }
            },
            BLOCK_E1_NEWLINE => lines.push(String::new()),
            _ => current.push(c),
        }
    }
    lines
}

impl TextBlock for AtomTable {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let (widths, heights) = self.grid(string_bounder);
        XDimension2D::new(widths.iter().sum(), heights.iter().sum())
    }

    fn draw_u(&self, ug: &UGraphic) {
        let (widths, heights) = self.grid(ug.string_bounder());
        let starts = |sizes: &[f64]| {
            std::iter::once(0.0)
                .chain(sizes.iter().scan(0.0, |total, size| {
                    *total += size;
                    Some(*total)
                }))
                .collect::<Vec<f64>>()
        };
        let (xs, ys) = (starts(&widths), starts(&heights));
        let fill = |color: &HColor, x: f64, y: f64, width: f64, height: f64| {
            ug.with_color(HColor::NONE)
                .with_backcolor(color.clone())
                .translated(x, y)
                .draw(&UShape::Rectangle(URectangle::new(width, height)));
        };
        let total_width = xs[xs.len() - 1];
        let total_height = ys[ys.len() - 1];
        for (row, line) in self.lines.iter().enumerate() {
            let height = ys[row + 1] - ys[row];
            if let Some(color) = &line.back_color {
                fill(color, 0.0, ys[row], total_width, height);
            }
            for (column, cell) in line.cells.iter().enumerate() {
                let width = xs[column + 1] - xs[column];
                let cell_ug = match &cell.back_color {
                    Some(color) => {
                        fill(color, xs[column], ys[row], width, height);
                        ug.with_backcolor(color.clone())
                    }
                    None => ug.clone(),
                };
                let dx = match cell.content.cell_alignment() {
                    HorizontalAlignment::Right => {
                        width - cell.content.calculate_dimension(ug.string_bounder()).width
                    }
                    HorizontalAlignment::Left | HorizontalAlignment::Center => 0.0,
                };
                cell.content
                    .draw_u(&cell_ug.translated(xs[column] + dx, ys[row]));
            }
        }
        let grid = ug.with_color(self.line_color.clone());
        for y in &ys {
            grid.translated(0.0, *y).draw(&UShape::Line {
                dx: total_width,
                dy: 0.0,
            });
        }
        for x in &xs {
            grid.translated(*x, 0.0).draw(&UShape::Line {
                dx: 0.0,
                dy: total_height,
            });
        }
    }
}

impl Atom for AtomTable {
    fn starting_altitude(&self, _string_bounder: &dyn StringBounder) -> f64 {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_lines_are_framed_by_bars() {
        assert!(is_table_line("|= a |= b |"));
        assert!(is_table_line("<#red,#blue>| a |"));
        assert!(!is_table_line("| a"));
        assert!(!is_table_line(" | a |"));
    }

    #[test]
    fn cells_break_at_written_newlines() {
        assert_eq!(cell_lines(r"a\nb\\c\td"), ["a", r"b\c\td"]);
        assert_eq!(cell_lines("a\u{E100}b"), ["a", "b"]);
    }

    #[test]
    fn leading_colours_give_back_and_line_colours() {
        let line = "<#lightyellow,red>| a |";
        assert_eq!(
            leading_color(line, 0),
            HColor::parse("lightyellow").unwrap()
        );
        assert_eq!(leading_color(line, 1), HColor::parse("red").unwrap());
        assert_eq!(leading_color("| a |", 0), None);
        assert_eq!(without_leading_color(line), "| a |");
    }
}
