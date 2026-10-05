//! Separator lines that span whatever contains them (PlantUML's `Stencil`, `UGraphicStencil` and
//! `UHorizontalLine`). A container that knows its extent puts a stencil on the drawing surface; a separator drawn
//! inside asks it where to start and end.

use std::rc::Rc;

use super::TextBlock;
use super::font::StringBounder;
use super::geom::UTranslate;
use super::shape::UShape;
use super::ugraphic::{UGraphic, UStroke};

/// The horizontal extent available at a height.
pub trait Stencil {
    fn starting_x(&self, string_bounder: &dyn StringBounder, y: f64) -> f64;
    fn ending_x(&self, string_bounder: &dyn StringBounder, y: f64) -> f64;
}

/// A separator: `-` and `=` draw solid lines (`=` two of them), `.` a dotted one; a title sits in its middle.
pub struct UHorizontalLine<'a> {
    pub style: char,
    pub title: Option<&'a dyn TextBlock>,
}

const DEFAULT_THICKNESS: f64 = 1.0;
const DOUBLE_LINE_GAP: f64 = 2.0;

impl UHorizontalLine<'_> {
    /// Draws the separator at height `y` of a surface placed where the stencil was set.
    pub(super) fn draw_line_internal(&self, ug: &UGraphic, stencil: &dyn Stencil, y: f64) {
        let ug_stroke = ug.with_stroke(self.stroke());
        let string_bounder = ug.string_bounder();
        let extent = |y| {
            (
                stencil.starting_x(string_bounder, y),
                stencil.ending_x(string_bounder, y),
            )
        };
        let Some(title) = self.title else {
            self.draw_lines(&ug_stroke, y, extent);
            return;
        };
        let title_width = title.calculate_dimension(string_bounder).width;
        let half = |(start, end): (f64, f64)| (end - start - title_width) / 2.0;
        self.draw_lines(&ug_stroke, y, |y| {
            let (start, end) = extent(y);
            (start, start + half((start, end)))
        });
        let (start, end) = extent(y);
        self.draw_title(ug, title, start, end, y);
        self.draw_lines(&ug_stroke, y, |y| {
            let (start, end) = extent(y);
            (end - half((start, end)), end)
        });
    }

    fn stroke(&self) -> UStroke {
        match self.style {
            '.' => UStroke {
                dash_visible: 1.0,
                dash_space: 2.0,
                thickness: 1.0,
            },
            '=' | '-' => UStroke::SIMPLE,
            _ => UStroke::with_thickness(DEFAULT_THICKNESS),
        }
    }

    fn draw_lines(&self, ug: &UGraphic, y: f64, extent: impl Fn(f64) -> (f64, f64)) {
        draw_line(ug, y, &extent);
        if self.style == '=' {
            draw_line(ug, y + DOUBLE_LINE_GAP, &extent);
        }
    }

    fn draw_title(&self, ug: &UGraphic, title: &dyn TextBlock, start: f64, end: f64, y: f64) {
        let dimension = title.calculate_dimension(ug.string_bounder());
        let x = start + (end - start - dimension.width) / 2.0;
        title.draw_u(&ug.translated(x, y - dimension.height / 2.0 - 0.5));
    }
}

fn draw_line(ug: &UGraphic, y: f64, extent: &impl Fn(f64) -> (f64, f64)) {
    let (start, end) = extent(y);
    ug.translated(start, y).draw(&UShape::Line {
        dx: end - start,
        dy: 0.0,
    });
}

/// A stencil set on a drawing surface, and where the surface was when it was set.
#[derive(Clone)]
pub(super) struct StencilFrame {
    pub stencil: Rc<dyn Stencil>,
    pub origin: UTranslate,
}
