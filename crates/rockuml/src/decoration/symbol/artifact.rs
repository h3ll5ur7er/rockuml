//! A box with a dog-eared sheet in its corner (PlantUML's `USymbolArtifact`).

use super::{BigContent, BigShape, Margin, SmallShape};
use crate::klimt::fashion::Fashion;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;

pub(super) struct USymbolArtifact;

fn draw_artifact(ug: &UGraphic, width_total: f64, height_total: f64, round_corner: f64) {
    const HEIGHT_SYMBOL: f64 = 14.0;
    const WIDTH_SYMBOL: f64 = 12.0;
    const CORNERSIZE: f64 = 6.0;
    ug.draw(&UShape::Rectangle(
        URectangle::new(width_total, height_total).rounded(round_corner),
    ));
    let sheet = vec![
        (0.0, 0.0),
        (0.0, HEIGHT_SYMBOL),
        (WIDTH_SYMBOL, HEIGHT_SYMBOL),
        (WIDTH_SYMBOL, CORNERSIZE),
        (WIDTH_SYMBOL - CORNERSIZE, 0.0),
        (0.0, 0.0),
    ];
    let x_symbol = width_total - WIDTH_SYMBOL - 5.0;
    let y_symbol = 5.0;
    ug.translated(x_symbol, y_symbol)
        .draw(&UShape::polygon(sheet));
    ug.translated(x_symbol + WIDTH_SYMBOL - CORNERSIZE, y_symbol)
        .draw(&UShape::Line {
            dx: 0.0,
            dy: CORNERSIZE,
        });
    ug.translated(x_symbol + WIDTH_SYMBOL, y_symbol + CORNERSIZE)
        .draw(&UShape::Line {
            dx: -CORNERSIZE,
            dy: 0.0,
        });
}

impl SmallShape for USymbolArtifact {
    fn margin(&self) -> Margin {
        Margin::new(10.0, 10.0 + 10.0, 10.0 + 3.0, 10.0)
    }

    fn draw_shape(&self, ug: &UGraphic, dimension: XDimension2D, fashion: &Fashion) {
        draw_artifact(ug, dimension.width, dimension.height, fashion.round_corner);
    }
}

impl BigShape for USymbolArtifact {
    fn draw_big(&self, ug: &UGraphic, content: &BigContent) {
        draw_artifact(
            ug,
            content.width,
            content.height,
            content.fashion.round_corner,
        );
        content.draw_centered_stereotype_and_title(ug, 2.0);
    }
}
