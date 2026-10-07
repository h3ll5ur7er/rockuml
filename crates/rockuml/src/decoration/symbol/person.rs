//! A head on a rounded body that holds the text (PlantUML's `USymbolPerson`).

use std::rc::Rc;

use super::{Margin, SmallContent};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{UEllipse, URectangle, UShape};
use crate::klimt::stencil::RectangleStencil;
use crate::klimt::ugraphic::UGraphic;

const MARGIN: Margin = Margin::new(10.0, 10.0, 10.0, 10.0);

pub(super) fn as_small(content: SmallContent) -> Box<dyn TextBlock> {
    Box::new(SmallPerson(content))
}

/// The head grows with the body's area.
fn head_size(dim_body: XDimension2D) -> f64 {
    let surface = dim_body.width * dim_body.height;
    surface.sqrt() * 0.42
}

struct SmallPerson(SmallContent);

impl SmallPerson {
    fn body_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        MARGIN.add_dimension(self.0.text_dimension(string_bounder))
    }
}

impl TextBlock for SmallPerson {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let body = self.body_dimension(string_bounder);
        body.delta(0.0, head_size(body))
    }

    fn draw_u(&self, ug: &UGraphic) {
        let dim_full = self.calculate_dimension(ug.string_bounder());
        let dim_body = self.body_dimension(ug.string_bounder());
        let ug = self
            .0
            .fashion
            .apply(ug)
            .with_stencil(Rc::new(RectangleStencil {
                width: dim_full.width,
            }));
        let head_size = head_size(dim_body);
        ug.translated((dim_body.width - head_size) / 2.0, 0.0)
            .draw(&UShape::Ellipse(UEllipse::new(head_size, head_size)));
        ug.translated(0.0, head_size).draw(&UShape::Rectangle(
            URectangle::new(dim_body.width, dim_body.height).rounded(head_size),
        ));
        self.0
            .text(self.0.stereo_alignment)
            .draw_u(&ug.translated(MARGIN.x1, MARGIN.y1 + head_size));
    }
}
