//! A cylinder standing upright (PlantUML's `USymbolDatabase`). Separators in its text are drawn as further
//! rims.

use std::rc::Rc;

use super::{BigContent, BigShape, Margin, SmallShape};
use crate::color::HColor;
use crate::klimt::fashion::Fashion;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{USegment, UShape};
use crate::klimt::stencil::{HorizontalLineDrawer, UHorizontalLine};
use crate::klimt::ugraphic::UGraphic;

pub(super) struct USymbolDatabase;

fn cubic(ctrl1: (f64, f64), ctrl2: (f64, f64), end: (f64, f64)) -> USegment {
    USegment::CubicTo { ctrl1, ctrl2, end }
}

/// The empty square past the bottom right corner makes room for the cylinder's curves.
fn draw_database(ug: &UGraphic, width: f64, height: f64) {
    ug.draw(&UShape::Path(vec![
        USegment::MoveTo(0.0, 10.0),
        cubic((0.0, 0.0), (width / 2.0, 0.0), (width / 2.0, 0.0)),
        cubic((width / 2.0, 0.0), (width, 0.0), (width, 10.0)),
        USegment::LineTo(width, height - 10.0),
        cubic(
            (width, height),
            (width / 2.0, height),
            (width / 2.0, height),
        ),
        cubic((width / 2.0, height), (0.0, height), (0.0, height - 10.0)),
        USegment::LineTo(0.0, 10.0),
    ]));
    ug.with_backcolor(HColor::NONE)
        .draw(&get_closing_path(width));
    ug.translated(width, height)
        .draw(&UShape::Empty(XDimension2D::new(10.0, 10.0)));
}

/// The front of the top rim.
fn get_closing_path(width: f64) -> UShape {
    UShape::Path(vec![
        USegment::MoveTo(0.0, 10.0),
        cubic((0.0, 20.0), (width / 2.0, 20.0), (width / 2.0, 20.0)),
        cubic((width / 2.0, 20.0), (width, 20.0), (width, 10.0)),
    ])
}

/// Draws separators as rims (`MyUGraphicDatabase`).
struct DatabaseLines {
    ending_x: f64,
}

impl HorizontalLineDrawer for DatabaseLines {
    fn draw_hline(&self, ug: &UGraphic, line: &UHorizontalLine, y: f64) {
        let closing = get_closing_path(self.ending_x);
        let ug = ug.translated(0.0, y);
        let rim = ug.with_stroke(line.stroke()).with_backcolor(HColor::NONE);
        rim.translated(0.0, -15.0).draw(&closing);
        if line.is_double() {
            rim.translated(0.0, -15.0 + 2.0).draw(&closing);
        }
        line.draw_title_internal(&ug, 0.0, self.ending_x, 0.0, true);
    }
}

impl SmallShape for USymbolDatabase {
    fn margin(&self) -> Margin {
        Margin::new(10.0, 10.0, 24.0, 5.0)
    }

    fn draw_shape(&self, ug: &UGraphic, dimension: XDimension2D, _fashion: &Fashion) {
        draw_database(ug, dimension.width, dimension.height);
    }

    fn text_surface(&self, ug: &UGraphic, dimension: XDimension2D) -> UGraphic {
        ug.with_horizontal_line_drawer(Rc::new(DatabaseLines {
            ending_x: dimension.width,
        }))
    }
}

impl BigShape for USymbolDatabase {
    fn draw_big(&self, ug: &UGraphic, content: &BigContent) {
        draw_database(ug, content.width, content.height);
        content.draw_centered_stereotype_and_title(ug, 2.0 + 20.0);
    }
}
