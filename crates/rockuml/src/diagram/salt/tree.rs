//! Trees and tree tables (`{T`): an indented first column with further cells beside it.

use std::cell::OnceCell;

use super::elements::{Element, TableStrategy, Text, color, widget_font};
use crate::color::HColor;
use crate::java;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;

pub(super) struct Tree {
    entries: Vec<TreeEntry>,
    strategy: TableStrategy,
    /// Computed on the first measurement: re-measuring nested trees on every call is exponential in
    /// their depth. Each export builds its own elements and measures them with one string bounder.
    layout: OnceCell<TreeLayout>,
}

/// The sizes of a tree's columns and rows.
struct TreeLayout {
    first_width: f64,
    other_widths: ListWidth,
    row_heights: Vec<f64>,
}

impl Tree {
    const MARGIN: f64 = 10.0;

    pub(super) fn new(strategy: TableStrategy) -> Self {
        Self {
            entries: Vec::new(),
            strategy,
            layout: OnceCell::new(),
        }
    }

    /// A row whose label's leading `+`s give its level.
    pub(super) fn add_entry(&mut self, text: &str) {
        let level = text.chars().take_while(|&c| c == '+').count();
        let label = Text::new(java::trim(&text[level..]), widget_font());
        self.entries.push(TreeEntry {
            level,
            first: label,
            others: Vec::new(),
        });
    }

    pub(super) fn add_cell_to_entry(&mut self, element: Box<dyn Element>) {
        if let Some(entry) = self.entries.last_mut() {
            entry.others.push(element);
        }
    }

    fn layout(&self, string_bounder: &dyn StringBounder) -> &TreeLayout {
        self.layout.get_or_init(|| TreeLayout {
            first_width: self.first_column_width(string_bounder),
            other_widths: self.other_widths(string_bounder),
            row_heights: self
                .entries
                .iter()
                .map(|entry| entry.row_height(string_bounder))
                .collect(),
        })
    }

    fn first_column_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.entries
            .iter()
            .map(|entry| entry.first_cell_dimension(string_bounder).width)
            .fold(0.0, f64::max)
    }

    fn other_widths(&self, string_bounder: &dyn StringBounder) -> ListWidth {
        self.entries
            .iter()
            .fold(ListWidth::default(), |widths, entry| {
                widths.merge_max(&entry.other_widths(string_bounder))
            })
    }
}

impl Element for Tree {
    fn preferred_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let layout = self.layout(string_bounder);
        let height = layout.row_heights.iter().sum();
        let mut others = layout.other_widths.total_width_with_margin(Self::MARGIN);
        if others > 0.0 {
            others += Self::MARGIN;
        }
        XDimension2D::new(layout.first_width + others + 2.0, height)
    }

    fn draw_u(&self, ug: &UGraphic, z_index: i32, _dimension: XDimension2D) {
        if z_index != 0 {
            return;
        }
        let ug = ug.with_color(HColor::BLACK);
        let TreeLayout {
            first_width,
            other_widths,
            row_heights,
        } = self.layout(ug.string_bounder());
        let mut cols = vec![0.0, first_width + Self::MARGIN / 2.0];
        for width in &other_widths.0 {
            cols.push(cols[cols.len() - 1] + (width + Self::MARGIN));
        }
        let mut rows = vec![0.0];
        let mut skeleton = Skeleton::default();
        let mut y = 0.0;
        for (entry, &height) in self.entries.iter().zip(row_heights) {
            entry.draw_first_cell(&ug, y);
            entry.draw_other_cells(&ug, first_width + Self::MARGIN, y, other_widths);
            skeleton.add(entry.x_delta() - 7.0, y + height / 2.0 - 1.0);
            y += height;
            rows.push(y);
        }
        let ug = ug.with_color(color("#8"));
        skeleton.draw(&ug);
        if self.strategy != TableStrategy::None {
            Grid2 {
                rows_start: &rows,
                cols_start: &cols,
                strategy: self.strategy,
            }
            .draw_u(&ug);
        }
    }
}

struct TreeEntry {
    level: usize,
    first: Text,
    others: Vec<Box<dyn Element>>,
}

impl TreeEntry {
    fn x_delta(&self) -> f64 {
        self.level as f64 * 10.0
    }

    fn first_cell_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.first
            .preferred_dimension(string_bounder)
            .delta(self.x_delta(), 0.0)
    }

    fn other_widths(&self, string_bounder: &dyn StringBounder) -> ListWidth {
        ListWidth(
            self.others
                .iter()
                .map(|element| element.preferred_dimension(string_bounder).width)
                .collect(),
        )
    }

    /// Tall enough for its tallest cell.
    fn row_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.others
            .iter()
            .map(|element| element.preferred_dimension(string_bounder).height)
            .fold(self.first_cell_dimension(string_bounder).height, f64::max)
    }

    fn draw_first_cell(&self, ug: &UGraphic, y: f64) {
        let dimension = self.first.preferred_dimension(ug.string_bounder());
        self.first
            .draw_u(&ug.translated(self.x_delta(), y), 0, dimension);
    }

    fn draw_other_cells(&self, ug: &UGraphic, mut x: f64, y: f64, widths: &ListWidth) {
        for (element, width) in self.others.iter().zip(&widths.0) {
            let dimension = element.preferred_dimension(ug.string_bounder());
            element.draw_u(&ug.translated(x, y), 0, dimension);
            x += width + Tree::MARGIN;
        }
    }
}

/// The widths of a row's cells after the first.
#[derive(Default)]
struct ListWidth(Vec<f64>);

impl ListWidth {
    fn merge_max(&self, other: &Self) -> Self {
        let width_at = |widths: &Self, index: usize| widths.0.get(index).copied().unwrap_or(0.0);
        let len = self.0.len().max(other.0.len());
        Self(
            (0..len)
                .map(|index| width_at(self, index).max(width_at(other, index)))
                .collect(),
        )
    }

    fn total_width_with_margin(&self, margin: f64) -> f64 {
        self.0.iter().fold(0.0, |total, width| {
            if total > 0.0 {
                total + margin + width
            } else {
                total + width
            }
        })
    }
}

/// The lines joining tree entries to their parents, with a box on every entry that has children.
#[derive(Default)]
struct Skeleton {
    entries: Vec<(f64, f64)>,
}

impl Skeleton {
    fn add(&mut self, x: f64, y: f64) {
        self.entries.push((x, y));
    }

    fn draw(&self, ug: &UGraphic) {
        for (index, &(x, y)) in self.entries.iter().enumerate() {
            if self
                .entries
                .get(index + 1)
                .is_some_and(|&(next_x, _)| next_x > x)
            {
                ug.translated(x, y)
                    .draw(&UShape::Rectangle(URectangle::new(2.0, 2.0)));
            }
            let parent = self.entries[..index]
                .iter()
                .rev()
                .find(|&&(parent_x, _)| parent_x < x);
            if let Some(&(parent_x, parent_y)) = parent {
                ug.translated(parent_x + 1.0, parent_y + 3.0)
                    .draw(&UShape::Line {
                        dx: 0.0,
                        dy: y - parent_y - 2.0,
                    });
                ug.translated(parent_x + 1.0, y + 1.0).draw(&UShape::Line {
                    dx: x - parent_x - 2.0,
                    dy: 0.0,
                });
            }
        }
    }
}

/// The lines of a tree table: around it, between its rows or between its columns.
struct Grid2<'a> {
    rows_start: &'a [f64],
    cols_start: &'a [f64],
    strategy: TableStrategy,
}

impl Grid2<'_> {
    fn draw_u(&self, ug: &UGraphic) {
        let (x_min, x_max) = (
            self.cols_start[0],
            self.cols_start[self.cols_start.len() - 1],
        );
        let (y_min, y_max) = (
            self.rows_start[0],
            self.rows_start[self.rows_start.len() - 1],
        );
        let hline = UShape::Line {
            dx: x_max - x_min,
            dy: 0.0,
        };
        let vline = UShape::Line {
            dx: 0.0,
            dy: y_max - y_min,
        };
        if matches!(
            self.strategy,
            TableStrategy::Outside | TableStrategy::OutsideWithTitle
        ) {
            ug.translated(x_min, y_min).draw(&hline);
            ug.translated(x_min, y_max).draw(&hline);
            ug.translated(x_min, y_min).draw(&vline);
            ug.translated(x_max, y_min).draw(&vline);
        }
        if matches!(
            self.strategy,
            TableStrategy::Horizontal | TableStrategy::All
        ) {
            for &y in self.rows_start {
                ug.translated(x_min, y).draw(&hline);
            }
        }
        if matches!(self.strategy, TableStrategy::Vertical | TableStrategy::All) {
            for &x in self.cols_start {
                ug.translated(x, y_min).draw(&vline);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_are_as_wide_as_their_widest_cell() {
        let merged = ListWidth(vec![5.0, 1.0]).merge_max(&ListWidth(vec![2.0, 3.0, 4.0]));
        assert_eq!(merged.0, [5.0, 3.0, 4.0]);
        assert_eq!(merged.total_width_with_margin(10.0), 32.0);
        assert_eq!(ListWidth::default().total_width_with_margin(10.0), 0.0);
    }
}
