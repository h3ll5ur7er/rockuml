//! A box with its title in a cut-off corner (PlantUML's `USymbolFrame`); also groups and partitions.

use super::{BigContent, BigShape, Margin, SmallShape};
use crate::color::HColor;
use crate::klimt::fashion::Fashion;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{UPath, URectangle, USegment, UShape};
use crate::klimt::ugraphic::UGraphic;

pub(super) struct USymbolFrame;

fn draw_frame(ug: &UGraphic, width: f64, height: f64, dim_title: XDimension2D, round_corner: f64) {
    ug.draw(&UShape::Rectangle(
        URectangle::new(width, height)
            .rounded(round_corner)
            .ignore_for_compression_on_x()
            .ignore_for_compression_on_y(),
    ));
    let (text_width, cornersize) = if dim_title.width == 0.0 {
        (width / 3.0, 7.0)
    } else {
        (dim_title.width + 10.0, 10.0)
    };
    let text_height = get_ypos(dim_title);
    let tab = UPath::new(vec![
        USegment::MoveTo(text_width, 0.0),
        USegment::LineTo(text_width, text_height - cornersize),
        USegment::LineTo(text_width - cornersize, text_height),
        USegment::LineTo(0.0, text_height),
    ]);
    ug.with_backcolor(HColor::NONE)
        .draw(&UShape::Path(tab.ignore_for_compression_on_x()));
}

/// The height of the title's corner.
fn get_ypos(dim_title: XDimension2D) -> f64 {
    if dim_title.width == 0.0 {
        12.0
    } else {
        dim_title.height + 3.0
    }
}

impl SmallShape for USymbolFrame {
    fn margin(&self) -> Margin {
        Margin::new(10.0 + 5.0, 20.0 + 5.0, 15.0 + 5.0, 5.0 + 5.0)
    }

    fn draw_shape(&self, ug: &UGraphic, dimension: XDimension2D, fashion: &Fashion) {
        draw_frame(
            ug,
            dimension.width,
            dimension.height,
            XDimension2D::default(),
            fashion.round_corner,
        );
    }
}

impl BigShape for USymbolFrame {
    fn draw_big(&self, ug: &UGraphic, content: &BigContent) {
        let string_bounder = ug.string_bounder();
        let dim_title = content.title.calculate_dimension(string_bounder);
        draw_frame(
            ug,
            content.width,
            content.height,
            dim_title,
            content.fashion.round_corner,
        );
        // A title with room around it goes as special text, which only some formats draw: PlantUML's "temporary
        // hack".
        let title_ug = ug.translated(3.0, 1.0);
        if content.width - dim_title.width < 25.0 {
            content.title.draw_u(&title_ug);
        } else {
            title_ug.draw_special_text(&*content.title);
        }
        let dim_stereo = content.stereotype.calculate_dimension(string_bounder);
        content.stereotype.draw_u(&ug.translated(
            4.0 + (content.width - dim_stereo.width) / 2.0,
            2.0 + get_ypos(dim_title),
        ));
    }
}
