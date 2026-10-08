//! A tile with one note beside it, pointing at it (PlantUML's `FtileWithNoteOpale`).

use std::rc::Rc;

use super::FtileWithNotes;
use super::note_sheet::NoteSheet;
use crate::diagram::activity3::{
    LinkRendering, NotePosition, NoteType, PositionedNote, SwimlaneId, SwimlaneSet,
};
use crate::direction::Direction;
use crate::ftile::{AbstractFtile, Ftile, FtileGeometry, Swimable};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XDimension2D, XPoint2D};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock, VerticalAlignment};
use crate::skin::SkinParam;
use crate::style::{PName, SName, StyleSignature, ValueReading};
use crate::svek::image::Opale;

/// The room between the note and the tile, which its callout crosses.
const SUPP_SPACE: f64 = 20.0;

pub(crate) struct FtileWithNoteOpale {
    base: AbstractFtile,
    tile: Rc<dyn Ftile>,
    opale: Opale<'static>,
    /// Floating notes point at nothing.
    with_link: bool,
    vertical_alignment: VerticalAlignment,
    note_position: NotePosition,
    swimlane_note: Option<SwimlaneId>,
}

impl FtileWithNoteOpale {
    /// `tile` with `notes` beside it: one note pointing at it, more notes as [`FtileWithNotes`].
    /// `skin_param` is the one `tile` is drawn with.
    ///
    /// # Panics
    ///
    /// Without notes, as PlantUML fails.
    pub(crate) fn create(
        tile: Rc<dyn Ftile>,
        skin_param: Rc<SkinParam>,
        notes: &[PositionedNote],
        with_link: bool,
        vertical_alignment: VerticalAlignment,
    ) -> Rc<dyn Ftile> {
        match notes {
            [] => panic!("a tile gets notes only when it has some"),
            [note] => Rc::new(Self::new(
                tile,
                skin_param,
                note,
                with_link,
                vertical_alignment,
            )),
            _ => Rc::new(FtileWithNotes::new(
                tile,
                skin_param,
                notes,
                vertical_alignment,
            )),
        }
    }

    fn new(
        tile: Rc<dyn Ftile>,
        skin_param: Rc<SkinParam>,
        note: &PositionedNote,
        with_link: bool,
        vertical_alignment: VerticalAlignment,
    ) -> Self {
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Note,
        ])
        .get_merged_style_with(
            &skin_param.current_style_builder(),
            note.stereotype.as_ref(),
        )
        .eventually_override_colors(&note.colors);
        let font_configuration = style.font_configuration();
        let alignment = skin_param.note_text_alignment(HorizontalAlignment::Left);
        let text = NoteSheet::new(
            &note.display,
            &font_configuration,
            alignment,
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
            tile,
            opale,
            with_link: with_link && note.type_ != NoteType::FloatingNote,
            vertical_alignment,
            note_position: note.note_position,
            swimlane_note: note.swimlane_note,
        }
    }

    fn get_translate(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_total = self.calculate_dimension_internal(string_bounder);
        let dim_note = self.opale.calculate_dimension(string_bounder);
        let dim_tile = self.tile.calculate_dimension(string_bounder);
        let y_for_ftile = (dim_total.height - dim_tile.get_height()) / 2.0;
        let marge = if self.note_position == NotePosition::Left {
            dim_note.width + SUPP_SPACE
        } else {
            0.0
        };
        UTranslate::new(marge, y_for_ftile)
    }

    fn get_translate_for_opale(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_total = self.calculate_dimension(string_bounder);
        let dim_note = self.opale.calculate_dimension(string_bounder);
        let y_for_note = if self.vertical_alignment == VerticalAlignment::Center {
            (dim_total.get_height() - dim_note.height) / 2.0
        } else {
            0.0
        };
        if self.note_position == NotePosition::Left {
            return UTranslate::new(0.0, y_for_note);
        }
        UTranslate::new(dim_total.get_width() - dim_note.width, y_for_note)
    }

    fn calculate_dimension_internal(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let dim_note = self.opale.calculate_dimension(string_bounder);
        let dim_tile = self.tile.calculate_dimension(string_bounder);
        let height = dim_note.height.max(dim_tile.get_height());
        XDimension2D::new(dim_tile.get_width() + dim_note.width + SUPP_SPACE, height)
    }

    /// Draws the note, pointing at the tile unless it floats.
    fn draw_note(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let ug = ug.apply(self.get_translate_for_opale(string_bounder));
        if !self.with_link {
            self.opale.draw_u(&ug);
            return;
        }
        let dim_note = self.opale.calculate_dimension(string_bounder);
        let middle = dim_note.height / 2.0;
        if self.note_position == NotePosition::Left {
            self.opale.draw_opale(
                &ug,
                Direction::Right,
                XPoint2D::new(dim_note.width, middle),
                XPoint2D::new(dim_note.width + SUPP_SPACE, middle),
            );
        } else {
            self.opale.draw_opale(
                &ug,
                Direction::Left,
                XPoint2D::new(0.0, middle),
                XPoint2D::new(-SUPP_SPACE, middle),
            );
        }
    }
}

/// The lane `ug` keeps, when it draws the tiles of one lane only: PlantUML's
/// `ug instanceof UGraphicInterceptorOneSwimlane`, then its `getSwimlane()`.
///
/// PLACEHOLDER until the swimlane layers are ported: no layer keeps one lane yet, so this answers `None`.
/// It becomes `ug.layer::<UGraphicInterceptorOneSwimlane>().map(UGraphicInterceptorOneSwimlane::get_swimlane)`.
fn swimlane_kept(_ug: &UGraphic) -> Option<SwimlaneId> {
    None
}

impl Swimable for FtileWithNoteOpale {
    fn get_swimlanes(&self) -> SwimlaneSet {
        let mut result = self.tile.get_swimlanes();
        if let Some(swimlane_note) = self.swimlane_note {
            result.insert(Some(swimlane_note));
        }
        result
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.tile.get_swimlane_in()
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.tile.get_swimlane_out()
    }
}

impl Ftile for FtileWithNoteOpale {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn get_in_link_rendering(&self) -> LinkRendering {
        self.tile.get_in_link_rendering()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base.calculate_dimension(|| {
            let dim_total = self.calculate_dimension_internal(string_bounder);
            let orig = self.tile.calculate_dimension(string_bounder);
            let translate = self.get_translate(string_bounder);
            let left = orig.get_left() + translate.dx;
            let in_y = orig.get_in_y() + translate.dy;
            if orig.has_point_out() {
                return FtileGeometry::from_dim_with_out(
                    dim_total,
                    left,
                    in_y,
                    orig.get_out_y() + translate.dy,
                );
            }
            FtileGeometry::from_dim(dim_total, left, in_y)
        })
    }

    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        vec![self.tile.clone()]
    }

    fn draw_u(&self, ug: &UGraphic) {
        let into_sw = swimlane_kept(ug);
        if into_sw.is_none() || self.swimlane_note.is_none() || into_sw == self.swimlane_note {
            self.draw_note(ug);
        }
        ug.apply(self.get_translate(ug.string_bounder()))
            .draw(&self.tile);
    }
}
