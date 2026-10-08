//! A box open at the top, with ledges on both sides (PlantUML's `USymbolStack`).

use super::{BigContent, BigShape, Margin, SmallShape};
use crate::color::HColor;
use crate::klimt::fashion::Fashion;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{URectangle, USegment, UShape};
use crate::klimt::ugraphic::UGraphic;

pub(super) struct USymbolStack;

/// The inside is filled without an outline; the outline leaves the top open.
fn draw_stack(ug: &UGraphic, width: f64, height: f64, round_corner: f64) {
    const BORDER: f64 = 15.0;
    ug.with_color(HColor::NONE)
        .translated(BORDER, 0.0)
        .draw(&UShape::Rectangle(
            URectangle::new(width - 2.0 * BORDER, height).rounded(round_corner),
        ));
    let outline = if round_corner == 0.0 {
        vec![
            USegment::MoveTo(0.0, 0.0),
            USegment::LineTo(BORDER, 0.0),
            USegment::LineTo(BORDER, height),
            USegment::LineTo(width - BORDER, height),
            USegment::LineTo(width - BORDER, 0.0),
            USegment::LineTo(width, 0.0),
        ]
    } else {
        let r = round_corner / 2.0;
        vec![
            USegment::MoveTo(0.0, 0.0),
            USegment::LineTo(BORDER - r, 0.0),
            USegment::arc_to((BORDER, r), r, true),
            USegment::LineTo(BORDER, height - r),
            USegment::arc_to((BORDER + r, height), r, false),
            USegment::LineTo(width - BORDER - r, height),
            USegment::arc_to((width - BORDER, height - r), r, false),
            USegment::LineTo(width - BORDER, r),
            USegment::arc_to((width - BORDER + r, 0.0), r, true),
            USegment::LineTo(width, 0.0),
        ]
    };
    ug.with_backcolor(HColor::NONE).draw(&UShape::path(outline));
}

impl SmallShape for USymbolStack {
    fn margin(&self) -> Margin {
        Margin::new(25.0, 25.0, 10.0, 10.0)
    }

    fn draw_shape(&self, ug: &UGraphic, dimension: XDimension2D, fashion: &Fashion) {
        draw_stack(ug, dimension.width, dimension.height, fashion.round_corner);
    }
}

impl BigShape for USymbolStack {
    fn draw_big(&self, ug: &UGraphic, content: &BigContent) {
        draw_stack(
            ug,
            content.width,
            content.height,
            content.fashion.round_corner,
        );
        content.draw_centered_stereotype_and_title(ug, 13.0);
    }
}
