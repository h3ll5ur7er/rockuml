//! A box, rounded or with cut corners (PlantUML's `USymbolRectangle`); also agents, `ArchiMate` elements and
//! components drawn as rectangles.

use super::{BigContent, BigShape, Margin, SmallShape};
use crate::klimt::HorizontalAlignment;
use crate::klimt::fashion::Fashion;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{URectangle, USegment, UShape};
use crate::klimt::ugraphic::UGraphic;

pub(super) struct USymbolRectangle;

const MARGIN: Margin = Margin::new(10.0, 10.0, 10.0, 10.0);

fn draw_rect(ug: &UGraphic, width: f64, height: f64, fashion: &Fashion) {
    let diagonal = fashion.diagonal_corner;
    if diagonal > 0.0 {
        ug.draw(&UShape::Path(vec![
            USegment::MoveTo(diagonal, 0.0),
            USegment::LineTo(width - diagonal, 0.0),
            USegment::LineTo(width, diagonal),
            USegment::LineTo(width, height - diagonal),
            USegment::LineTo(width - diagonal, height),
            USegment::LineTo(diagonal, height),
            USegment::LineTo(0.0, height - diagonal),
            USegment::LineTo(0.0, diagonal),
            USegment::LineTo(diagonal, 0.0),
        ]));
    } else {
        ug.draw(&UShape::Rectangle(
            URectangle::new(width, height).rounded(fashion.round_corner),
        ));
    }
}

impl SmallShape for USymbolRectangle {
    fn margin(&self) -> Margin {
        MARGIN
    }

    fn draw_shape(&self, ug: &UGraphic, dimension: XDimension2D, fashion: &Fashion) {
        draw_rect(ug, dimension.width, dimension.height, fashion);
    }

    fn text_alignment(&self, stereo_alignment: HorizontalAlignment) -> HorizontalAlignment {
        stereo_alignment
    }
}

impl BigShape for USymbolRectangle {
    fn draw_big(&self, ug: &UGraphic, content: &BigContent) {
        draw_rect(ug, content.width, content.height, &content.fashion);
        content.draw_stereotype_top_and_title(ug, MARGIN, BigContent::aligned_title_x);
    }
}
