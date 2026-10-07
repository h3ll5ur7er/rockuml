//! A sheet with a folded corner (PlantUML's `USymbolFile`).

use super::{BigContent, BigShape, Margin, SmallShape};
use crate::klimt::fashion::Fashion;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{USegment, UShape};
use crate::klimt::ugraphic::UGraphic;

pub(super) struct USymbolFile;

const CORNERSIZE: f64 = 10.0;

/// A quarter circle of radius `radius` to `end`, turning left.
fn arc_to(end: (f64, f64), radius: f64) -> USegment {
    USegment::ArcTo {
        radius: (radius, radius),
        x_axis_rotation: 0.0,
        large_arc: false,
        sweep: false,
        end,
    }
}

fn draw_file(ug: &UGraphic, width: f64, height: f64, round_corner: f64) {
    if round_corner == 0.0 {
        ug.draw(&UShape::Polygon(vec![
            (0.0, 0.0),
            (0.0, height),
            (width, height),
            (width, CORNERSIZE),
            (width - CORNERSIZE, 0.0),
            (0.0, 0.0),
        ]));
    } else {
        let r = round_corner / 2.0;
        ug.draw(&UShape::Path(vec![
            USegment::MoveTo(0.0, r),
            USegment::LineTo(0.0, height - r),
            arc_to((r, height), r),
            USegment::LineTo(width - r, height),
            arc_to((width, height - r), r),
            USegment::LineTo(width, CORNERSIZE),
            USegment::LineTo(width - CORNERSIZE, 0.0),
            USegment::LineTo(r, 0.0),
            arc_to((0.0, r), r),
        ]));
    }
    let mut fold = vec![USegment::MoveTo(width - CORNERSIZE, 0.0)];
    if round_corner == 0.0 {
        fold.push(USegment::LineTo(width - CORNERSIZE, CORNERSIZE));
    } else {
        let r = round_corner / 2.0;
        fold.push(USegment::LineTo(width - CORNERSIZE, CORNERSIZE - r));
        fold.push(arc_to((width - CORNERSIZE + r, CORNERSIZE), r));
    }
    fold.push(USegment::LineTo(width, CORNERSIZE));
    ug.draw(&UShape::Path(fold));
}

impl SmallShape for USymbolFile {
    fn margin(&self) -> Margin {
        Margin::new(10.0, 10.0, 10.0, 10.0)
    }

    fn draw_shape(&self, ug: &UGraphic, dimension: XDimension2D, fashion: &Fashion) {
        draw_file(ug, dimension.width, dimension.height, fashion.round_corner);
    }
}

impl BigShape for USymbolFile {
    fn draw_big(&self, ug: &UGraphic, content: &BigContent) {
        draw_file(
            ug,
            content.width,
            content.height,
            content.fashion.round_corner,
        );
        content.draw_centered_stereotype_and_title(ug, 2.0);
    }
}
