//! A tick of the bit scale above the rows (PlantUML's `PacketIndicator`).

use crate::color::{Colors, HColor};
use crate::creole::{CreoleMode, Display};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{USegment, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::style::Style;

/// The height of a full tick.
const V_LINE_FULL: f64 = 32.0;
/// The height of a short tick.
const V_LINE_SHORT: f64 = V_LINE_FULL / 2.0;

/// A full or short tick at a bit position, numbered or not.
pub(super) struct PacketIndicator {
    pub(super) full: bool,
    pub(super) numbered: bool,
    pub(super) bit_number: i32,
}

impl PacketIndicator {
    fn number_tb(&self, style: &Style, skin: &SkinParam) -> impl TextBlock {
        Display::create([self.bit_number.to_string()]).create0(
            &style.font_configuration(),
            HorizontalAlignment::Left,
            skin,
            0.0,
            CreoleMode::NoCreole,
        )
    }

    fn v_line_height(&self) -> f64 {
        if self.full { V_LINE_FULL } else { V_LINE_SHORT }
    }

    /// The number above the tick, always measured so that unnumbered ticks line up (`calculateDimension`).
    pub(super) fn calculate_dimension(
        &self,
        style: &Style,
        skin: &SkinParam,
        string_bounder: &dyn StringBounder,
    ) -> XDimension2D {
        self.number_tb(style, skin)
            .calculate_dimension(string_bounder)
            .merge_top_bottom(XDimension2D::new(0.0, self.v_line_height()))
    }

    /// The number centred on the tick, the tick below it.
    pub(super) fn draw_u(&self, ug: &UGraphic, style: &Style, skin: &SkinParam) {
        let number_tb = self.number_tb(style, skin);
        let number_dim = number_tb.calculate_dimension(ug.string_bounder());
        let line_offset_y = if self.full { 0.0 } else { V_LINE_SHORT };
        if self.numbered {
            number_tb.draw_u(&ug.translated(-number_dim.width / 2.0, line_offset_y));
        }
        draw_v_line(
            &ug.translated(0.0, number_dim.height),
            style,
            line_offset_y,
            self.v_line_height(),
        );
    }
}

/// A tick from `y` down `height` (`createVLineTextBlock`, drawing nwdiag's `VerticalLine` with nothing to skip).
fn draw_v_line(ug: &UGraphic, style: &Style, y: f64, height: f64) {
    let line_color = style.symbol_context(&Colors::default()).fore_color;
    ug.with_color(line_color)
        .with_stroke(style.stroke())
        .with_backcolor(HColor::NONE)
        .draw(&UShape::path(vec![
            USegment::MoveTo(0.0, y),
            USegment::LineTo(0.0, y + height),
        ]));
}
