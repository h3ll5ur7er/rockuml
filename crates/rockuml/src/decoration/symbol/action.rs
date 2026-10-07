//! A box pointing right (PlantUML's `USymbolAction`).

use super::{BigContent, BigShape, Margin, SmallShape};
use crate::klimt::HorizontalAlignment;
use crate::klimt::fashion::Fashion;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::UGraphic;

pub(super) struct USymbolAction;

fn draw_action(ug: &UGraphic, width: f64, height: f64) {
    ug.draw(&UShape::Polygon(vec![
        (0.0, 0.0),
        (width - 10.0, 0.0),
        (width, height / 2.0),
        (width - 10.0, height),
        (0.0, height),
    ]));
}

impl SmallShape for USymbolAction {
    fn margin(&self) -> Margin {
        Margin::new(10.0, 20.0, 10.0, 10.0)
    }

    fn draw_shape(&self, ug: &UGraphic, dimension: XDimension2D, _fashion: &Fashion) {
        draw_action(ug, dimension.width, dimension.height);
    }

    fn text_alignment(&self, stereo_alignment: HorizontalAlignment) -> HorizontalAlignment {
        stereo_alignment
    }
}

impl BigShape for USymbolAction {
    fn draw_big(&self, ug: &UGraphic, content: &BigContent) {
        draw_action(ug, content.width, content.height);
        content.draw_title_then_stereotype(ug);
    }
}
