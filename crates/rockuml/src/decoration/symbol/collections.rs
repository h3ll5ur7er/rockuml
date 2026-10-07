//! A box in front of another (PlantUML's `USymbolCollections`).

use super::{BigContent, BigShape, Margin, SmallShape};
use crate::klimt::HorizontalAlignment;
use crate::klimt::fashion::Fashion;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;

pub(super) struct USymbolCollections;

const MARGIN: Margin = Margin::new(10.0, 10.0, 10.0, 10.0);
const DELTA_COLLECTION: f64 = 4.0;

fn draw_collections(ug: &UGraphic, width: f64, height: f64, round_corner: f64) {
    let small = UShape::Rectangle(
        URectangle::new(width - DELTA_COLLECTION, height - DELTA_COLLECTION).rounded(round_corner),
    );
    ug.translated(DELTA_COLLECTION, DELTA_COLLECTION)
        .draw(&small);
    ug.draw(&small);
}

impl SmallShape for USymbolCollections {
    fn margin(&self) -> Margin {
        MARGIN
    }

    fn draw_shape(&self, ug: &UGraphic, dimension: XDimension2D, fashion: &Fashion) {
        draw_collections(ug, dimension.width, dimension.height, fashion.round_corner);
    }

    fn text_alignment(&self, stereo_alignment: HorizontalAlignment) -> HorizontalAlignment {
        stereo_alignment
    }

    /// Centred on the front box.
    fn text_position(&self) -> (f64, f64) {
        (
            MARGIN.x1 - DELTA_COLLECTION / 2.0,
            MARGIN.y1 - DELTA_COLLECTION / 2.0,
        )
    }
}

impl BigShape for USymbolCollections {
    fn draw_big(&self, ug: &UGraphic, content: &BigContent) {
        draw_collections(
            ug,
            content.width,
            content.height,
            content.fashion.round_corner,
        );
        content.draw_stereotype_top_and_title(ug, MARGIN, BigContent::centered_x);
    }
}
