//! A box, its big form with a line under the title (PlantUML's `USymbolCard`).

use super::{BigContent, BigShape, Margin, SmallShape};
use crate::klimt::fashion::Fashion;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;

pub(super) struct USymbolCard;

/// A line at `top` unless it is 0.
fn draw_card(ug: &UGraphic, width: f64, height: f64, top: f64, round_corner: f64) {
    ug.draw(&UShape::Rectangle(
        URectangle::new(width, height).rounded(round_corner),
    ));
    if top != 0.0 {
        ug.translated(0.0, top)
            .draw(&UShape::Line { dx: width, dy: 0.0 });
    }
}

impl SmallShape for USymbolCard {
    fn margin(&self) -> Margin {
        Margin::new(10.0, 10.0, 3.0, 3.0)
    }

    fn draw_shape(&self, ug: &UGraphic, dimension: XDimension2D, fashion: &Fashion) {
        draw_card(
            ug,
            dimension.width,
            dimension.height,
            0.0,
            fashion.round_corner,
        );
    }
}

impl BigShape for USymbolCard {
    fn draw_big(&self, ug: &UGraphic, content: &BigContent) {
        let string_bounder = ug.string_bounder();
        let dim_stereo = content.stereotype.calculate_dimension(string_bounder);
        let dim_title = content.title.calculate_dimension(string_bounder);
        draw_card(
            ug,
            content.width,
            content.height,
            dim_title.height + dim_stereo.height + 4.0,
            content.fashion.round_corner,
        );
        content.draw_centered_stereotype_and_title(ug, 2.0);
    }
}
