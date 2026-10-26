//! A field drawn in a row (PlantUML's `PacketBlock`).

use std::rc::Rc;

use crate::color::Colors;
use crate::creole::{CreoleMode, Display};
use crate::klimt::blocks::TextBlockVertical;
use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::stencil::RectangleStencil;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::skin::component::TextBlockEmpty;
use crate::style::Style;

/// The room above and below a label.
const V_MARGIN: f64 = 10.0;

/// A field's share of a row, in bits across and in `node_height`s down. PlantUML also marks the edges where a
/// field continues on another row, but draws them like any other.
pub(super) struct PacketBlock {
    pub(super) width: i32,
    pub(super) height: i32,
    label: String,
}

impl PacketBlock {
    pub(super) fn new(width: i32, height: i32, label: &str) -> Self {
        Self {
            width,
            height,
            label: label.to_owned(),
        }
    }

    pub(super) fn draw_width(&self, scale: f64) -> f64 {
        f64::from(self.width) * scale
    }

    fn draw_height(&self, scale: f64) -> f64 {
        f64::from(self.height) * scale
    }

    fn label_text_block(label: &str, style: &Style, skin: &SkinParam) -> Box<dyn TextBlock> {
        Box::new(Display::with_newlines(label).create0(
            &style.font_configuration(),
            HorizontalAlignment::Center,
            skin,
            0.0,
            CreoleMode::SimpleLine,
        ))
    }

    /// The label, cut short with `...` until it fits the field (`getLabelTextBlockAbbr`). PlantUML measures
    /// the label as plain text, its markup and `\n` included.
    fn label_text_block_abbr(
        &self,
        style: &Style,
        skin: &SkinParam,
        string_bounder: &dyn StringBounder,
        bit_width: f64,
    ) -> Box<dyn TextBlock> {
        const PAD: &str = "...";
        let font = style.ufont();
        let pad_dim = string_bounder.calculate_dimension(&font, PAD);
        let mut label_dim = string_bounder.calculate_dimension(&font, &self.label);
        let req_width = self.draw_width(bit_width);
        if label_dim.width < req_width {
            return Self::label_text_block(&self.label, style, skin);
        }
        let units: Vec<u16> = self.label.encode_utf16().collect();
        let mut abbr = self.label.clone();
        for i in (1..=units.len()).rev() {
            if label_dim.width + pad_dim.width < req_width {
                break;
            }
            abbr = String::from_utf16_lossy(&units[..i]) + PAD;
            label_dim = string_bounder.calculate_dimension(&font, &abbr);
        }
        Self::label_text_block(&abbr, style, skin)
    }

    /// The field's box with its label centred in it (`getShapeTextBlock`).
    pub(super) fn shape_text_block(
        &self,
        style: &Style,
        skin: &SkinParam,
        string_bounder: &dyn StringBounder,
        bit_width: f64,
        bit_height: f64,
    ) -> PacketShape {
        let req_width = self.draw_width(bit_width);
        let label = self.label_text_block_abbr(style, skin, string_bounder, bit_width);
        let label_height = label.calculate_dimension(string_bounder).height;
        let total_spare =
            (self.draw_height(bit_height) - label_height - V_MARGIN - V_MARGIN).max(0.0);
        let top_spare = total_spare / 2.0;
        PacketShape {
            fashion: style.symbol_context(&Colors::default()),
            req_width,
            label,
            label_height,
            top_spare,
            bottom_spare: total_spare - top_spare,
        }
    }
}

pub(super) struct PacketShape {
    fashion: Fashion,
    req_width: f64,
    label: Box<dyn TextBlock>,
    label_height: f64,
    top_spare: f64,
    bottom_spare: f64,
}

impl TextBlock for PacketShape {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(
            self.req_width,
            self.top_spare + self.label_height + self.bottom_spare + V_MARGIN + V_MARGIN,
        )
    }

    fn draw_u(&self, ug: &UGraphic) {
        let dim = self.calculate_dimension(ug.string_bounder());
        if dim.width == 0.0 || dim.height == 0.0 {
            return;
        }
        let ug = ug.with_stencil_stroke(
            Rc::new(RectangleStencil { width: dim.width }),
            UStroke::with_thickness(1.0),
        );
        let ug = self.fashion.apply(&ug);
        ug.draw(&UShape::Rectangle(
            URectangle::new(dim.width, dim.height)
                .rounded(self.fashion.round_corner)
                .with_delta_shadow(self.fashion.delta_shadow),
        ));
        let spacer = |height| -> Box<dyn TextBlock> {
            Box::new(TextBlockEmpty {
                dimension: XDimension2D::new(self.req_width, height),
            })
        };
        TextBlockVertical::new(
            vec![
                spacer(self.top_spare),
                Box::new(self.label.as_ref()),
                spacer(self.bottom_spare),
            ],
            HorizontalAlignment::Center,
        )
        .draw_u(&ug.translated(0.0, V_MARGIN));
    }
}
