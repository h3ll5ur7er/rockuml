//! Separator lines that span whatever contains them (PlantUML's `Stencil`, `AbstractUGraphicHorizontalLine`,
//! `UGraphicStencil` and `UHorizontalLine`). A container that knows its extent puts a separator drawer on the
//! drawing surface; a separator drawn inside asks it to draw the line.

use std::rc::Rc;

use super::TextBlock;
use super::font::StringBounder;
use super::geom::UTranslate;
use super::shape::{URectangle, UShape};
use super::ugraphic::{UGraphic, UStroke};

/// The horizontal extent available at a height.
pub trait Stencil {
    fn starting_x(&self, string_bounder: &dyn StringBounder, y: f64) -> f64;
    fn ending_x(&self, string_bounder: &dyn StringBounder, y: f64) -> f64;
}

/// How a container draws the separators drawn inside it (`AbstractUGraphicHorizontalLine.drawHline`).
pub trait HorizontalLineDrawer {
    /// Draws `line` at height `y` of `ug`, a surface placed where the drawer was set.
    fn draw_hline(&self, ug: &UGraphic, line: &UHorizontalLine, y: f64);
}

/// Separators spanning a stencil (`UGraphicStencil`).
pub(crate) struct UGraphicStencil {
    pub stencil: Rc<dyn Stencil>,
    /// What separators without a style of their own are drawn with: the outline of the shape the stencil is.
    pub default_stroke: Option<UStroke>,
}

impl HorizontalLineDrawer for UGraphicStencil {
    fn draw_hline(&self, ug: &UGraphic, line: &UHorizontalLine, y: f64) {
        line.draw_line_internal(ug, self.stencil.as_ref(), y, self.default_stroke);
    }
}

/// The full width of a box (`UGraphicStencil.getRectangleStencil`).
pub(crate) struct RectangleStencil {
    pub width: f64,
}

impl Stencil for RectangleStencil {
    fn starting_x(&self, _string_bounder: &dyn StringBounder, _y: f64) -> f64 {
        0.0
    }

    fn ending_x(&self, _string_bounder: &dyn StringBounder, _y: f64) -> f64 {
        self.width
    }
}

/// A separator: `-` and `=` draw solid lines (`=` two of them), `.` a dotted one, others a line of
/// `default_thickness`; a title sits in its middle.
pub struct UHorizontalLine<'a> {
    pub style: char,
    pub title: Option<&'a dyn TextBlock>,
    pub default_thickness: f64,
    /// How far inside the stencil the line starts and ends.
    pub skip: f64,
}

const DOUBLE_LINE_GAP: f64 = 2.0;

impl UHorizontalLine<'_> {
    /// Draws the separator at height `y` of a surface placed where the stencil was set.
    pub(crate) fn draw_line_internal(
        &self,
        ug: &UGraphic,
        stencil: &dyn Stencil,
        y: f64,
        default_stroke: Option<UStroke>,
    ) {
        let stroke = match default_stroke {
            Some(stroke) if self.style == '\0' => stroke,
            _ => self.stroke(),
        };
        let ug_stroke = ug.with_stroke(stroke);
        let string_bounder = ug.string_bounder();
        let extent = |y| {
            (
                stencil.starting_x(string_bounder, y) + self.skip,
                stencil.ending_x(string_bounder, y) - self.skip,
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
        self.draw_title_internal(ug, start, end, y, false);
        self.draw_lines(&ug_stroke, y, |y| {
            let (start, end) = extent(y);
            (end - half((start, end)), end)
        });
    }

    pub(crate) fn is_double(&self) -> bool {
        self.style == '='
    }

    pub(crate) fn stroke(&self) -> UStroke {
        match self.style {
            '.' => UStroke {
                dash_visible: 1.0,
                dash_space: 2.0,
                thickness: 1.0,
            },
            '=' | '-' => UStroke::SIMPLE,
            _ => UStroke::with_thickness(self.default_thickness),
        }
    }

    fn draw_lines(&self, ug: &UGraphic, y: f64, extent: impl Fn(f64) -> (f64, f64)) {
        draw_line(ug, y, &extent);
        if self.is_double() {
            draw_line(ug, y + DOUBLE_LINE_GAP, &extent);
        }
    }

    /// Centres the title on the line, half a pixel up; `clear_area` first frames the title in the line's stroke,
    /// filled with the background, over whatever the container drew there.
    pub(crate) fn draw_title_internal(
        &self,
        ug: &UGraphic,
        start: f64,
        end: f64,
        y: f64,
        clear_area: bool,
    ) {
        let Some(title) = self.title else {
            return;
        };
        let dimension = title.calculate_dimension(ug.string_bounder());
        let x = start + (end - start - dimension.width) / 2.0;
        let ug = ug.translated(x, y - dimension.height / 2.0 - 0.5);
        if clear_area {
            ug.with_stroke(self.stroke())
                .draw(&UShape::Rectangle(URectangle::new(
                    dimension.width,
                    dimension.height,
                )));
        }
        title.draw_u(&ug);
    }
}

fn draw_line(ug: &UGraphic, y: f64, extent: &impl Fn(f64) -> (f64, f64)) {
    let (start, end) = extent(y);
    ug.translated(start, y).draw(&UShape::Line {
        dx: end - start,
        dy: 0.0,
    });
}

/// A separator drawer set on a drawing surface, and where the surface was when it was set.
#[derive(Clone)]
pub(super) struct StencilFrame {
    pub drawer: Rc<dyn HorizontalLineDrawer>,
    pub origin: UTranslate,
}
