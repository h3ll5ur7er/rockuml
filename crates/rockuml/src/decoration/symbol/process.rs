//! A chevron (PlantUML's `USymbolProcess`).

use super::{BigContent, BigShape, Margin, SmallShape};
use crate::klimt::HorizontalAlignment;
use crate::klimt::fashion::Fashion;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::UGraphic;

pub(super) struct USymbolProcess;

fn draw_process(ug: &UGraphic, width: f64, height: f64) {
    ug.draw(&UShape::Polygon(vec![
        (0.0, 0.0),
        (width - 10.0, 0.0),
        (width, height / 2.0),
        (width - 10.0, height),
        (0.0, height),
        (10.0, height / 2.0),
    ]));
}

impl SmallShape for USymbolProcess {
    fn margin(&self) -> Margin {
        Margin::new(20.0, 20.0, 10.0, 10.0)
    }

    fn draw_shape(&self, ug: &UGraphic, dimension: XDimension2D, _fashion: &Fashion) {
        draw_process(ug, dimension.width, dimension.height);
    }

    fn text_alignment(&self, stereo_alignment: HorizontalAlignment) -> HorizontalAlignment {
        stereo_alignment
    }
}

impl BigShape for USymbolProcess {
    fn draw_big(&self, ug: &UGraphic, content: &BigContent) {
        draw_process(ug, content.width, content.height);
        content.draw_title_then_stereotype(ug);
    }
}
