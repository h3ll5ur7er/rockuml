//! The grid drawn as a table: its lines, then each cell centred in its line and column (PlantUML's
//! `GridArray` and the drawing of `ConnectorPuzzleEmpty`).

use std::rc::Rc;

use super::element::{hline, vline};
use super::grid::{Connections, Placeable, Where};
use crate::color::HColor;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;

/// The room around each cell.
const MARGIN: f64 = 30.0;

pub(super) struct GridArray {
    data: Vec<Vec<Option<Box<dyn TextBlock>>>>,
    cols: usize,
}

impl GridArray {
    pub(super) fn new(placeables: Vec<Vec<Option<Placeable>>>, skin_param: &Rc<SkinParam>) -> Self {
        let cols = placeables.first().map_or(0, Vec::len);
        let data = placeables
            .into_iter()
            .map(|line| {
                line.into_iter()
                    .map(|cell| {
                        cell.map(|placeable| match placeable {
                            Placeable::Element(element) => element.to_text_block(skin_param),
                            Placeable::Puzzle(connections) => {
                                Box::new(ConnectorPuzzleEmpty(connections)) as Box<dyn TextBlock>
                            }
                        })
                    })
                    .collect()
            })
            .collect();
        Self { data, cols }
    }

    fn get_height_of_line(&self, string_bounder: &dyn StringBounder, line: usize) -> f64 {
        self.data[line]
            .iter()
            .flatten()
            .map(|cell| cell.calculate_dimension(string_bounder).height)
            .fold(0.0, f64::max)
    }

    fn get_width_of_col(&self, string_bounder: &dyn StringBounder, col: usize) -> f64 {
        self.data
            .iter()
            .filter_map(|line| line[col].as_ref())
            .map(|cell| cell.calculate_dimension(string_bounder).width)
            .fold(0.0, f64::max)
    }

    fn total_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        (0..self.data.len())
            .map(|l| self.get_height_of_line(string_bounder, l) + MARGIN)
            .fold(0.0, |sum, height| sum + height)
    }

    fn total_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        (0..self.cols)
            .map(|c| self.get_width_of_col(string_bounder, c) + MARGIN)
            .fold(0.0, |sum, width| sum + width)
    }

    /// A line above each line and left of each column.
    fn draw_internal_grid(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let height_max = self.total_height(string_bounder);
        let width_max = self.total_width(string_bounder);
        let ug = ug.with_color(HColor::BLACK);
        let mut y = 0.0;
        for l in 0..self.data.len() {
            ug.translated(0.0, y).draw(&hline(width_max));
            y += self.get_height_of_line(string_bounder, l) + MARGIN;
        }
        let mut x = 0.0;
        for c in 0..self.cols {
            ug.translated(x, 0.0).draw(&vline(height_max));
            x += self.get_width_of_col(string_bounder, c) + MARGIN;
        }
    }
}

impl TextBlock for GridArray {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(
            self.total_width(string_bounder),
            self.total_height(string_bounder),
        )
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        self.draw_internal_grid(ug);
        let mut dy = 0.0;
        for (l, line) in self.data.iter().enumerate() {
            let mut dx = 0.0;
            let height_of_line = self.get_height_of_line(string_bounder, l);
            for (c, cell) in line.iter().enumerate() {
                let width_of_col = self.get_width_of_col(string_bounder, c);
                if let Some(cell) = cell {
                    let dim = cell.calculate_dimension(string_bounder);
                    cell.draw_u(&ug.translated(
                        dx + (width_of_col + MARGIN - dim.width) / 2.0,
                        dy + (height_of_line + MARGIN - dim.height) / 2.0,
                    ));
                }
                dx += width_of_col + MARGIN;
            }
            dy += height_of_line + MARGIN;
        }
    }
}

/// A piece of connector: blue strokes from the centre of a 20 × 20 cell to its connected sides.
struct ConnectorPuzzleEmpty(Connections);

impl TextBlock for ConnectorPuzzleEmpty {
    fn calculate_dimension(&self, _: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(20.0, 20.0)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let ug = ug.with_color(HColor::BLUE);
        for w in Where::VALUES {
            if self.0.have(w) {
                let (x, y, line) = match w {
                    Where::West => (0.0, 10.0, hline(10.0)),
                    Where::East => (10.0, 10.0, hline(10.0)),
                    Where::North => (10.0, 0.0, vline(10.0)),
                    Where::South => (10.0, 10.0, vline(10.0)),
                };
                ug.translated(x, y).draw(&line);
            }
        }
    }
}
