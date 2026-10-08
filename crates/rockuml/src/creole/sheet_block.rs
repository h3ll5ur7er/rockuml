use std::borrow::Cow;
use std::rc::Rc;

use super::{Atom, Sheet, Stripe, fission};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, MinMax, XDimension2D};
use crate::klimt::stencil::Stencil;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::{HorizontalAlignment, TextBlock};

/// Lays a sheet out: atoms side by side on their stripe's baseline, stripes stacked top to bottom.
pub(crate) struct SheetBlock1 {
    sheet: Sheet,
    padding: ClockwiseTopRightBottomLeft,
    /// How wide a line may grow before it wraps; 0 never wraps.
    max_width: f64,
}

#[derive(Clone, Copy)]
struct Position {
    x: f64,
    y: f64,
    dimension: XDimension2D,
}

struct Layout<'s> {
    /// The sheet's stripes, wrapped.
    stripes: Cow<'s, [Stripe]>,
    /// Per stripe, per atom.
    positions: Vec<Vec<Position>>,
    min_max: MinMax,
}

impl SheetBlock1 {
    pub(crate) fn new(sheet: Sheet, padding: ClockwiseTopRightBottomLeft) -> Self {
        Self {
            sheet,
            padding,
            max_width: 0.0,
        }
    }

    #[must_use]
    pub(crate) fn wrapped_at(self, max_width: f64) -> Self {
        Self { max_width, ..self }
    }

    fn stripes(&self, string_bounder: &dyn StringBounder) -> Cow<'_, [Stripe]> {
        if self.max_width == 0.0 {
            return Cow::Borrowed(&self.sheet.stripes);
        }
        Cow::Owned(
            self.sheet
                .stripes
                .iter()
                .flat_map(|stripe| fission::split(stripe, self.max_width, string_bounder))
                .filter(|stripe| !stripe.atoms.is_empty())
                .collect(),
        )
    }

    /// How a single line of text aligns itself in a table cell; longer sheets align left.
    pub(super) fn cell_alignment(&self) -> HorizontalAlignment {
        match self.sheet.stripes.as_slice() {
            [only] => only.cell_alignment,
            _ => HorizontalAlignment::Left,
        }
    }

    fn layout(&self, string_bounder: &dyn StringBounder) -> Layout<'_> {
        let stripes = self.stripes(string_bounder);
        let mut positions = Vec::with_capacity(stripes.len());
        let mut widths = Vec::with_capacity(stripes.len());
        let mut min_max = MinMax::from_origin();
        let mut y = 0.0;
        for stripe in stripes.iter() {
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
        for ((stripe, sea), width) in stripes.iter().zip(&mut positions).zip(widths) {
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
        Layout {
            stripes,
            positions,
            min_max,
        }
    }
}

/// Places atoms left to right on a common baseline, the topmost at `top`. Returns them and their width.
fn sea(
    atoms: &[Rc<dyn Atom>],
    top: f64,
    string_bounder: &dyn StringBounder,
) -> (Vec<Position>, f64) {
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
        for (stripe, positions) in layout.stripes.iter().zip(&layout.positions) {
            for (atom, position) in stripe.atoms.iter().zip(positions) {
                atom.draw_u(&ug.translated(position.x, position.y));
            }
        }
    }
}

/// The width of the sheet.
impl Stencil for SheetBlock1 {
    fn starting_x(&self, _string_bounder: &dyn StringBounder, _y: f64) -> f64 {
        // PlantUML's `-marginX1` of a sheet without margins, whose sign can reach the output.
        -0.0
    }

    fn ending_x(&self, string_bounder: &dyn StringBounder, _y: f64) -> f64 {
        self.calculate_dimension(string_bounder).width
    }
}

/// A laid-out sheet whose separators span a stencil (PlantUML's `SheetBlock2`), and inside a border, the border's
/// padding as well.
pub(crate) struct SheetBlock2 {
    block: Rc<SheetBlock1>,
    stencil: Rc<dyn Stencil>,
    /// What separators without a style of their own are drawn with.
    default_stroke: Option<UStroke>,
}

impl SheetBlock2 {
    /// Separators spanning the sheet itself.
    pub(crate) fn new(block: SheetBlock1) -> Self {
        let block = Rc::new(block);
        Self {
            stencil: block.clone(),
            block,
            default_stroke: None,
        }
    }

    pub(crate) fn with_stencil(
        block: Rc<SheetBlock1>,
        stencil: Rc<dyn Stencil>,
        default_stroke: UStroke,
    ) -> Self {
        Self {
            block,
            stencil,
            default_stroke: Some(default_stroke),
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
        let stencil = Rc::new(EnlargedStencil {
            stencil: self.stencil.clone(),
            left,
            right,
        });
        let ug = match self.default_stroke {
            Some(stroke) => ug.with_stencil_stroke(stencil, stroke),
            None => ug.with_stencil(stencil),
        };
        self.block.draw_u(&ug);
    }
}

/// A stencil widened on each side (`SheetBlock2.enlargeMe`).
struct EnlargedStencil {
    stencil: Rc<dyn Stencil>,
    left: f64,
    right: f64,
}

impl Stencil for EnlargedStencil {
    fn starting_x(&self, string_bounder: &dyn StringBounder, y: f64) -> f64 {
        self.stencil.starting_x(string_bounder, y) - self.left
    }

    fn ending_x(&self, string_bounder: &dyn StringBounder, y: f64) -> f64 {
        self.stencil.ending_x(string_bounder, y) + self.right
    }
}
