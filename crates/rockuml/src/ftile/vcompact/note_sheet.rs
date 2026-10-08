//! The text of a note of an activity diagram, as its tiles lay it out for an [`Opale`](crate::svek::image::Opale).

use crate::creole::{CreoleMode, CreoleParser, Display, SheetBlock1, SheetBlock2};
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::XDimension2D;
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::svek::image::{MARGIN_X1, MARGIN_X2};

/// A note's text, wrapped at the style's width, whose separators span the note's outline: PlantUML's
/// `SheetBlock2` with the tile, or an anonymous stencil, reaching from 6 before the text to 15 after it.
pub(super) struct NoteSheet {
    sheet: SheetBlock2,
}

impl NoteSheet {
    /// `skinParam.sheet(fc, alignment, CreoleMode.FULL).createSheet(note)`, which unlike `Display.create`
    /// leaves out the note's own alignment.
    pub(super) fn new(
        note: &Display,
        font_configuration: &FontConfiguration,
        alignment: HorizontalAlignment,
        wrap_width: f64,
        skin_param: &SkinParam,
    ) -> Self {
        let sheet = CreoleParser::with_mode(
            font_configuration.clone(),
            alignment,
            CreoleMode::Full,
            skin_param,
        )
        .create_display_sheet(note, font_configuration);
        Self {
            sheet: SheetBlock2::new(
                SheetBlock1::new(sheet, skin_param.get_padding()).wrapped_at(wrap_width),
            ),
        }
    }
}

impl TextBlock for NoteSheet {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.sheet.calculate_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.sheet.draw_in_padding(ug, MARGIN_X1, MARGIN_X2);
    }
}
