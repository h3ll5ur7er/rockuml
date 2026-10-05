use std::rc::Rc;

use super::{Atom, Sheet};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, MinMax, XDimension2D};
use crate::klimt::stencil::Stencil;
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};

/// Lays a sheet out: atoms side by side on their stripe's baseline, stripes stacked top to bottom.
pub struct SheetBlock1 {
    sheet: Sheet,
    padding: ClockwiseTopRightBottomLeft,
}

#[derive(Clone, Copy)]
struct Position {
    x: f64,
    y: f64,
    dimension: XDimension2D,
}

struct Layout {
    /// Per stripe, per atom.
    positions: Vec<Vec<Position>>,
    min_max: MinMax,
}

impl SheetBlock1 {
    pub fn new(sheet: Sheet, padding: ClockwiseTopRightBottomLeft) -> Self {
        Self { sheet, padding }
    }

    fn layout(&self, string_bounder: &dyn StringBounder) -> Layout {
        let mut positions = Vec::with_capacity(self.sheet.stripes.len());
        let mut widths = Vec::with_capacity(self.sheet.stripes.len());
        let mut min_max = MinMax::from_origin();
        let mut y = 0.0;
        for stripe in &self.sheet.stripes {
            if stripe.atoms.is_empty() {
                positions.push(Vec::new());
                widths.push(0.0);
                continue;
            }
            let (sea, width) = sea(&stripe.atoms, y, string_bounder);
            for position in &sea {
                min_max = min_max.add_point(
                    position.x + position.dimension.width,
                    position.y + position.dimension.height,
                );
            }
            y += max_y(&sea) - min_y(&sea);
            positions.push(sea);
            widths.push(width);
        }

        let max_width = widths.iter().copied().fold(0.0, f64::max);
        for ((stripe, sea), width) in self.sheet.stripes.iter().zip(&mut positions).zip(widths) {
            let shared_by = match stripe.cell_alignment {
                HorizontalAlignment::Left => continue,
                HorizontalAlignment::Center => 2.0,
                HorizontalAlignment::Right => 1.0,
            };
            if max_width > width {
                for position in sea {
                    position.x += (max_width - width) / shared_by;
                }
            }
        }
        Layout { positions, min_max }
    }
}

/// Places atoms left to right on a common baseline, the topmost at `top`. Returns them and their width.
fn sea(
    atoms: &[Box<dyn Atom>],
    top: f64,
    string_bounder: &dyn StringBounder,
) -> (Vec<Position>, f64) {
    debug_assert!(!atoms.is_empty(), "a sea needs atoms to find its top");
    let mut x = 0.0;
    let mut positions: Vec<Position> = atoms
        .iter()
        .map(|atom| {
            let dimension = atom.calculate_dimension(string_bounder);
            let position = Position {
                x,
                y: -dimension.height + atom.starting_altitude(string_bounder),
                dimension,
            };
            x += dimension.width;
            position
        })
        .collect();
    let delta = top - min_y(&positions);
    for position in &mut positions {
        position.y += delta;
    }
    (positions, x)
}

fn min_y(positions: &[Position]) -> f64 {
    positions
        .iter()
        .map(|position| position.y)
        .fold(f64::MAX, f64::min)
}

fn max_y(positions: &[Position]) -> f64 {
    positions
        .iter()
        .map(|position| position.y + position.dimension.height)
        .fold(-f64::MAX, f64::max)
}

impl TextBlock for SheetBlock1 {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        // PlantUML widens by the vertical padding too.
        let vertical_padding = self.padding.top + self.padding.bottom;
        self.layout(string_bounder)
            .min_max
            .dimension()
            .delta(vertical_padding, vertical_padding)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let layout = self.layout(ug.string_bounder());
        let ug = ug.translated(self.padding.left, self.padding.top);
        for (stripe, positions) in self.sheet.stripes.iter().zip(&layout.positions) {
            for (atom, position) in stripe.atoms.iter().zip(positions) {
                atom.draw_u(&ug.translated(position.x, position.y));
            }
        }
    }
}

/// A laid-out sheet whose separators span it (PlantUML's `SheetBlock2`), and inside a border, the border's padding.
pub struct SheetBlock2 {
    block: Rc<SheetBlock1>,
}

impl SheetBlock2 {
    pub fn new(block: SheetBlock1) -> Self {
        Self {
            block: Rc::new(block),
        }
    }
}

impl TextBlock for SheetBlock2 {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.block.calculate_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.draw_in_padding(ug, 0.0, 0.0);
    }

    fn draw_in_padding(&self, ug: &UGraphic, left: f64, right: f64) {
        let stencil = SheetStencil {
            sheet: self.block.clone(),
            left,
            right,
        };
        self.block.draw_u(&ug.with_stencil(Rc::new(stencil)));
    }
}

/// The width of a sheet, widened on each side.
struct SheetStencil {
    sheet: Rc<SheetBlock1>,
    left: f64,
    right: f64,
}

impl Stencil for SheetStencil {
    fn starting_x(&self, _string_bounder: &dyn StringBounder, _y: f64) -> f64 {
        -self.left
    }

    fn ending_x(&self, string_bounder: &dyn StringBounder, _y: f64) -> f64 {
        self.sheet.calculate_dimension(string_bounder).width + self.right
    }
}
