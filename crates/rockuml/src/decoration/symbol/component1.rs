//! A box with two small boxes across its left side, the UML 1 component notation (PlantUML's
//! `USymbolComponent1`). Its big form is the UML 2 one.

use super::{Margin, SmallShape};
use crate::klimt::fashion::Fashion;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;

pub(super) struct USymbolComponent1;

impl SmallShape for USymbolComponent1 {
    fn margin(&self) -> Margin {
        Margin::new(10.0, 10.0, 10.0, 10.0)
    }

    fn draw_shape(&self, ug: &UGraphic, dimension: XDimension2D, fashion: &Fashion) {
        ug.draw(&UShape::Rectangle(
            URectangle::new(dimension.width, dimension.height).rounded(fashion.round_corner),
        ));
        let small = UShape::Rectangle(URectangle::new(10.0, 5.0));
        ug.translated(-5.0, 5.0).draw(&small);
        ug.translated(-5.0, dimension.height - 10.0).draw(&small);
    }
}
