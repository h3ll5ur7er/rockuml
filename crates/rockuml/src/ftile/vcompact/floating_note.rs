//! A note drawn on its own, in the note style of a diagram (PlantUML's `FloatingNote`).

use super::note_sheet::note_opale;
use crate::creole::Display;
use crate::direction::Direction;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{XDimension2D, XPoint2D};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::style::{SName, StyleSignature};
use crate::svek::image::Opale;

pub(crate) struct FloatingNote {
    opale: Opale<'static>,
}

impl FloatingNote {
    /// The note in the style of `sname`'s notes.
    pub(crate) fn create(note: &Display, skin_param: &SkinParam, sname: SName) -> Self {
        let style = StyleSignature::of(&[SName::Root, SName::Element, sname, SName::Note])
            .get_merged_style(&skin_param.current_style_builder());
        let opale = note_opale(
            note,
            &style,
            skin_param.get_default_text_alignment(HorizontalAlignment::Left),
            skin_param,
        );
        Self { opale }
    }

    /// Draws the note reaching out from its side `strategy` near `pp1` to `pp2` (`createOpale`, `setOpale`
    /// and `drawU`).
    pub(crate) fn draw_opale(
        &self,
        ug: &UGraphic,
        strategy: Direction,
        pp1: XPoint2D,
        pp2: XPoint2D,
    ) {
        self.opale.draw_opale(ug, strategy, pp1, pp2);
    }
}

impl TextBlock for FloatingNote {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.opale.calculate_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.opale.draw_u(ug);
    }
}
