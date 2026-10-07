//! Text without a frame (PlantUML's `USymbolLabel`).

use super::{BigContent, BigShape, Margin, SmallShape};
use crate::klimt::HorizontalAlignment;
use crate::klimt::fashion::Fashion;
use crate::klimt::geom::XDimension2D;
use crate::klimt::ugraphic::UGraphic;

pub(super) struct USymbolLabel;

const MARGIN: Margin = Margin::new(10.0, 10.0, 10.0, 10.0);

impl SmallShape for USymbolLabel {
    fn margin(&self) -> Margin {
        MARGIN
    }

    fn draw_shape(&self, _ug: &UGraphic, _dimension: XDimension2D, _fashion: &Fashion) {}

    fn text_alignment(&self, stereo_alignment: HorizontalAlignment) -> HorizontalAlignment {
        stereo_alignment
    }
}

impl BigShape for USymbolLabel {
    fn draw_big(&self, ug: &UGraphic, content: &BigContent) {
        content.draw_stereotype_top_and_title(ug, MARGIN, BigContent::aligned_title_x);
    }
}
