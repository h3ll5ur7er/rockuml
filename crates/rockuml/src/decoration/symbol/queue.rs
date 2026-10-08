//! A cylinder lying on its side (PlantUML's `USymbolQueue`). Separators in its text end on its rim.

use std::rc::Rc;

use super::{BigContent, BigShape, Margin, SmallShape};
use crate::color::HColor;
use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{USegment, UShape};
use crate::klimt::stencil::Stencil;
use crate::klimt::ugraphic::UGraphic;

pub(super) struct USymbolQueue;

const DX: f64 = 5.0;

fn cubic(ctrl1: (f64, f64), ctrl2: (f64, f64), end: (f64, f64)) -> USegment {
    USegment::CubicTo { ctrl1, ctrl2, end }
}

fn draw_queue(ug: &UGraphic, width: f64, height: f64) {
    ug.draw(&UShape::path(vec![
        USegment::MoveTo(DX, 0.0),
        USegment::LineTo(width - DX, 0.0),
        cubic((width, 0.0), (width, height / 2.0), (width, height / 2.0)),
        cubic((width, height / 2.0), (width, height), (width - DX, height)),
        USegment::LineTo(DX, height),
        cubic((0.0, height), (0.0, height / 2.0), (0.0, height / 2.0)),
        cubic((0.0, height / 2.0), (0.0, 0.0), (DX, 0.0)),
    ]));
    ug.with_backcolor(HColor::NONE)
        .draw(&get_closing_path(width, height));
}

/// The front of the right end.
fn get_closing_path(width: f64, height: f64) -> UShape {
    UShape::path(vec![
        USegment::MoveTo(width - DX, 0.0),
        cubic(
            (width - DX * 2.0, 0.0),
            (width - DX * 2.0, height / 2.0),
            (width - DX * 2.0, height / 2.0),
        ),
        cubic(
            (width - DX * 2.0, height),
            (width - DX, height),
            (width - DX, height),
        ),
    ])
}

/// Separators end on the front of the right end, approximated by two straight lines (`MyUGraphicQueue`).
struct QueueStencil {
    x1: f64,
    x2: f64,
    full_height: f64,
}

impl Stencil for QueueStencil {
    fn starting_x(&self, _string_bounder: &dyn StringBounder, _y: f64) -> f64 {
        0.0
    }

    fn ending_x(&self, _string_bounder: &dyn StringBounder, y: f64) -> f64 {
        let half_height = self.full_height / 2.0;
        let factor2 = if y <= half_height {
            1.0 - (y / half_height)
        } else {
            (y - half_height) / half_height
        };
        (self.x2 - self.x1) * factor2 + self.x1
    }
}

impl SmallShape for USymbolQueue {
    fn margin(&self) -> Margin {
        Margin::new(5.0, 15.0, 5.0, 5.0)
    }

    fn draw_shape(&self, ug: &UGraphic, dimension: XDimension2D, _fashion: &Fashion) {
        draw_queue(ug, dimension.width, dimension.height);
    }

    fn text_surface(&self, ug: &UGraphic, dimension: XDimension2D) -> UGraphic {
        ug.with_stencil(Rc::new(QueueStencil {
            x1: dimension.width - 2.0 * DX,
            x2: dimension.width - DX,
            full_height: dimension.height,
        }))
    }
}

impl BigShape for USymbolQueue {
    fn draw_big(&self, ug: &UGraphic, content: &BigContent) {
        draw_queue(ug, content.width, content.height);
        content.draw_centered_stereotype_and_title(ug, 2.0);
    }
}
