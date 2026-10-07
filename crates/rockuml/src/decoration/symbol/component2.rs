//! A box with a component icon in its corner, the UML 2 component notation (PlantUML's `USymbolComponent2`).

use super::{BigContent, BigShape, Margin, SmallShape};
use crate::klimt::fashion::Fashion;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;

pub(super) struct USymbolComponent2;

fn draw_component2(ug: &UGraphic, width_total: f64, height_total: f64, round_corner: f64) {
    let small = UShape::Rectangle(URectangle::new(15.0, 10.0));
    let tiny = UShape::Rectangle(URectangle::new(4.0, 2.0));
    ug.draw(&UShape::Rectangle(
        URectangle::new(width_total, height_total).rounded(round_corner),
    ));
    ug.translated(width_total - 20.0, 5.0).draw(&small);
    ug.translated(width_total - 22.0, 7.0).draw(&tiny);
    ug.translated(width_total - 22.0, 11.0).draw(&tiny);
}

impl SmallShape for USymbolComponent2 {
    fn margin(&self) -> Margin {
        Margin::new(10.0 + 5.0, 20.0 + 5.0, 15.0 + 5.0, 5.0 + 5.0)
    }

    fn draw_shape(&self, ug: &UGraphic, dimension: XDimension2D, fashion: &Fashion) {
        draw_component2(ug, dimension.width, dimension.height, fashion.round_corner);
    }
}

impl BigShape for USymbolComponent2 {
    fn draw_big(&self, ug: &UGraphic, content: &BigContent) {
        draw_component2(
            ug,
            content.width,
            content.height,
            content.fashion.round_corner,
        );
        content.draw_centered_stereotype_and_title(ug, 13.0);
    }
}
