//! A box with very round corners (PlantUML's `USymbolStorage`).

use super::{BigContent, BigShape, Margin, SmallShape};
use crate::klimt::fashion::Fashion;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;

pub(super) struct USymbolStorage;

fn draw_storage(ug: &UGraphic, width: f64, height: f64, shadowing: f64) {
    ug.draw(&UShape::Rectangle(
        URectangle::new(width, height)
            .rounded(70.0)
            .with_delta_shadow(shadowing),
    ));
}

impl SmallShape for USymbolStorage {
    fn margin(&self) -> Margin {
        Margin::new(10.0, 10.0, 10.0, 10.0)
    }

    fn draw_shape(&self, ug: &UGraphic, dimension: XDimension2D, fashion: &Fashion) {
        draw_storage(ug, dimension.width, dimension.height, fashion.delta_shadow);
    }
}

impl BigShape for USymbolStorage {
    fn draw_big(&self, ug: &UGraphic, content: &BigContent) {
        draw_storage(
            ug,
            content.width,
            content.height,
            content.fashion.delta_shadow,
        );
        let string_bounder = ug.string_bounder();
        let dim_stereo = content.stereotype.calculate_dimension(string_bounder);
        content
            .stereotype
            .draw_u(&ug.translated(content.centered_x(dim_stereo.width), 5.0));
        let dim_title = content.title.calculate_dimension(string_bounder);
        content
            .title
            .draw_u(&ug.translated(content.centered_x(dim_title.width), 7.0 + dim_stereo.height));
    }
}
