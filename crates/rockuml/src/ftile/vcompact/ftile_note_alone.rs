//! A note without a tile, as a list starting with a note has (PlantUML's `FtileNoteAlone`).

use std::rc::Rc;

use super::note_sheet::NoteSheet;
use crate::color::{ColorType, Colors};
use crate::creole::Display;
use crate::diagram::activity3::{SwimlaneId, SwimlaneSet};
use crate::ftile::{AbstractFtile, Ftile, FtileGeometry, Swimable};
use crate::klimt::font::StringBounder;
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::skin::font_param::FontParam;
use crate::style::{PName, SName, StyleSignature, ValueReading};
use crate::svek::image::Opale;

pub(crate) struct FtileNoteAlone {
    base: AbstractFtile,
    opale: Opale<'static>,
    /// Whether arrows leave it: notes do, floating notes do not.
    with_out_point: bool,
    swimlane: Option<SwimlaneId>,
}

impl FtileNoteAlone {
    /// The note's own `colors` only colour its text, which is in the font of `skinparam note`.
    pub(crate) fn new(
        note: &Display,
        skin_param: Rc<SkinParam>,
        colors: &Colors,
        with_out_point: bool,
        swimlane: Option<SwimlaneId>,
    ) -> Self {
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Note,
        ])
        .get_merged_style(&skin_param.current_style_builder());
        let mut font_configuration = skin_param.get_font_configuration(FontParam::Note, None);
        if let Some(color) = colors.get(ColorType::Text) {
            font_configuration = font_configuration.with_color(color.clone());
        }
        let text = NoteSheet::new(
            note,
            &font_configuration,
            skin_param.get_default_text_alignment(HorizontalAlignment::Left),
            style.wrap_width(),
            &skin_param,
        );
        let opale = Opale::new(
            style.value(PName::LineColor).as_color(),
            style.value(PName::BackGroundColor).as_color(),
            Box::new(text),
            style.stroke(),
            0.0,
        );
        Self {
            base: AbstractFtile::new(skin_param),
            opale,
            with_out_point,
            swimlane,
        }
    }
}

impl Swimable for FtileNoteAlone {
    fn get_swimlanes(&self) -> SwimlaneSet {
        self.swimlane.map(Some).into_iter().collect()
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.swimlane
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.swimlane
    }
}

impl Ftile for FtileNoteAlone {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base.calculate_dimension(|| {
            let dim_total = self.opale.calculate_dimension(string_bounder);
            if self.with_out_point {
                return FtileGeometry::from_dim_with_out(
                    dim_total,
                    dim_total.width / 2.0,
                    0.0,
                    dim_total.height,
                );
            }
            FtileGeometry::from_dim(dim_total, dim_total.width / 2.0, 0.0)
        })
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.opale.draw_u(ug);
    }
}
