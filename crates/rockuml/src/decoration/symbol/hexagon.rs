//! A hexagon (PlantUML's `USymbolHexagon`). Its small form only places the text: the element draws the
//! hexagon around it.

use super::{BigContent, BigShape, Margin, SmallContent};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{USegment, UShape};
use crate::klimt::ugraphic::UGraphic;

pub(super) struct USymbolHexagon;

const MARGIN_Y: f64 = 5.0;

pub(super) fn as_small(content: SmallContent) -> Box<dyn TextBlock> {
    Box::new(SmallHexagon(content))
}

struct SmallHexagon(SmallContent);

impl TextBlock for SmallHexagon {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let full = self.0.text_dimension(string_bounder);
        XDimension2D::new(full.width * 2.0, full.height + 2.0 * MARGIN_Y)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let dimension = self.calculate_dimension(ug.string_bounder());
        self.0
            .text(self.0.stereo_alignment)
            .draw_u(&ug.translated(dimension.width / 4.0, MARGIN_Y));
    }
}

fn draw_rect(ug: &UGraphic, width: f64, height: f64) {
    let dx = width / 8.0;
    ug.draw(&UShape::Path(vec![
        USegment::MoveTo(0.0, height / 2.0),
        USegment::LineTo(dx, 0.0),
        USegment::LineTo(width - dx, 0.0),
        USegment::LineTo(width, height / 2.0),
        USegment::LineTo(width - dx, height),
        USegment::LineTo(dx, height),
        USegment::LineTo(0.0, height / 2.0),
    ]));
}

impl BigShape for USymbolHexagon {
    fn draw_big(&self, ug: &UGraphic, content: &BigContent) {
        draw_rect(ug, content.width, content.height);
        content.draw_stereotype_top_and_title(
            ug,
            Margin::new(10.0, 10.0, 10.0, 10.0),
            BigContent::aligned_title_x,
        );
    }
}
