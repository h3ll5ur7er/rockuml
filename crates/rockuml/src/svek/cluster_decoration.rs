//! A cluster drawn as the big form of its symbol (PlantUML's `ClusterDecoration`).

use crate::color::HColor;
use crate::decoration::symbol::{Block, PackageStyle, USymbol};
use crate::klimt::fashion::Fashion;
use crate::klimt::geom::RectangleArea;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::{HorizontalAlignment, TextBlock};

pub(crate) struct ClusterDecoration {
    default_stroke: UStroke,
    /// `None` for package styles without a big form, which PlantUML cannot draw.
    symbol: Option<USymbol>,
    title: Block,
    stereo: Block,
    rectangle_area: RectangleArea,
}

impl ClusterDecoration {
    pub(crate) fn new(
        style: PackageStyle,
        symbol: Option<USymbol>,
        title: Block,
        stereo: Block,
        rectangle_area: RectangleArea,
        stroke: UStroke,
    ) -> Self {
        Self {
            default_stroke: stroke,
            symbol: symbol.or_else(|| style.to_u_symbol()),
            title,
            stereo,
            rectangle_area,
        }
    }

    #[expect(clippy::too_many_arguments, reason = "PlantUML's drawU")]
    pub(crate) fn draw_u(
        &self,
        ug: &UGraphic,
        back_color: HColor,
        border_color: HColor,
        round_corner: f64,
        title_alignment: HorizontalAlignment,
        stereo_alignment: HorizontalAlignment,
        diagonal_corner: f64,
    ) {
        if let Some(as_big) = self.get_text_block(
            back_color,
            border_color,
            round_corner,
            title_alignment,
            stereo_alignment,
            diagonal_corner,
        ) {
            let position = self.rectangle_area.get_position();
            as_big.draw_u(&ug.translated(position.dx, position.dy));
        }
    }

    /// The symbol around the cluster's area; `None` for a style without a symbol.
    pub(crate) fn get_text_block(
        &self,
        back_color: HColor,
        border_color: HColor,
        round_corner: f64,
        title_alignment: HorizontalAlignment,
        stereo_alignment: HorizontalAlignment,
        diagonal_corner: f64,
    ) -> Option<Box<dyn TextBlock>> {
        let symbol_context = Fashion::new(back_color, border_color)
            .with_stroke(self.default_stroke)
            .with_corner(round_corner, diagonal_corner);
        Some(self.symbol?.as_big(
            self.title.clone(),
            title_alignment,
            self.stereo.clone(),
            self.rectangle_area.get_width(),
            self.rectangle_area.get_height(),
            symbol_context,
            stereo_alignment,
        ))
    }
}
