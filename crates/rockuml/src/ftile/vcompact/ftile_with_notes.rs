//! A tile with notes left and right of it, pointing at nothing (PlantUML's `FtileWithNotes`).

use std::cell::OnceCell;
use std::rc::Rc;

use super::note_sheet::NoteSheet;
use crate::diagram::activity3::{
    LinkRendering, NotePosition, PositionedNote, SwimlaneId, SwimlaneSet,
};
use crate::ftile::{Ftile, FtileGeometry, Swimable};
use crate::klimt::blocks::{TextBlockMarged, TextBlockVertical};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, UTranslate, XDimension2D};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock, VerticalAlignment};
use crate::skin::SkinParam;
use crate::skin::component::TextBlockEmpty;
use crate::style::{PName, SName, StyleSignature, ValueReading};
use crate::svek::image::Opale;

/// The room around each note.
const NOTE_MARGIN: f64 = 10.0;

pub(crate) struct FtileWithNotes {
    tile: Rc<dyn Ftile>,
    left: Box<dyn TextBlock>,
    right: Box<dyn TextBlock>,
    vertical_alignment: VerticalAlignment,
    cached_geometry: OnceCell<FtileGeometry>,
}

impl FtileWithNotes {
    /// `tile` with `notes` stacked on their side.
    pub(crate) fn new(
        tile: Rc<dyn Ftile>,
        notes: &[PositionedNote],
        vertical_alignment: VerticalAlignment,
    ) -> Self {
        let skin_param = tile.skin_param();
        let mut left: Option<Box<dyn TextBlock>> = None;
        let mut right: Option<Box<dyn TextBlock>> = None;
        for note in notes {
            let style = StyleSignature::of(&[
                SName::Root,
                SName::Element,
                SName::ActivityDiagram,
                SName::Note,
            ])
            .get_merged_style(&skin_param.current_style_builder())
            .eventually_override_colors(&note.colors);
            let text = NoteSheet::new(
                &note.display,
                &style.font_configuration(),
                skin_param.get_default_text_alignment(HorizontalAlignment::Left),
                style.wrap_width(),
                skin_param,
            );
            let opale = Opale::new(
                style.value(PName::LineColor).as_color(),
                style.value(PName::BackGroundColor).as_color(),
                Box::new(text),
                style.stroke(),
                0.0,
            );
            let opale_marged: Box<dyn TextBlock> = Box::new(TextBlockMarged::new(
                opale,
                ClockwiseTopRightBottomLeft::same(NOTE_MARGIN),
            ));
            let side = if note.note_position == NotePosition::Left {
                &mut left
            } else {
                &mut right
            };
            *side = Some(match side.take() {
                None => opale_marged,
                Some(above) => Box::new(TextBlockVertical::new(
                    vec![above, opale_marged],
                    HorizontalAlignment::Center,
                )),
            });
        }
        let or_empty = |side: Option<Box<dyn TextBlock>>| {
            side.unwrap_or_else(|| Box::new(TextBlockEmpty::default()))
        };
        Self {
            left: or_empty(left),
            right: or_empty(right),
            tile,
            vertical_alignment,
            cached_geometry: OnceCell::new(),
        }
    }

    /// How far down a block `height` high goes beside the others.
    fn y_delta(&self, string_bounder: &dyn StringBounder, height: f64) -> f64 {
        if self.vertical_alignment == VerticalAlignment::Top {
            return 0.0;
        }
        (self.calculate_dimension_internal(string_bounder).height - height) / 2.0
    }

    fn get_translate(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_tile = self.tile.calculate_dimension(string_bounder);
        let x_delta = self.left.calculate_dimension(string_bounder).width;
        UTranslate::new(x_delta, self.y_delta(string_bounder, dim_tile.get_height()))
    }

    fn get_translate_for_left(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_left = self.left.calculate_dimension(string_bounder);
        UTranslate::new(0.0, self.y_delta(string_bounder, dim_left.height))
    }

    fn get_translate_for_right(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_total = self.calculate_dimension_internal(string_bounder);
        let dim_right = self.right.calculate_dimension(string_bounder);
        UTranslate::new(
            dim_total.width - dim_right.width,
            self.y_delta(string_bounder, dim_right.height),
        )
    }

    fn calculate_dimension_internal(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let dim_tile = self.tile.calculate_dimension(string_bounder);
        let dim_left = self.left.calculate_dimension(string_bounder);
        let dim_right = self.right.calculate_dimension(string_bounder);
        let height = dim_left
            .height
            .max(dim_right.height)
            .max(dim_tile.get_height());
        XDimension2D::new(
            dim_tile.get_width() + dim_left.width + dim_right.width,
            height,
        )
    }
}

impl Swimable for FtileWithNotes {
    fn get_swimlanes(&self) -> SwimlaneSet {
        self.tile.get_swimlanes()
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.tile.get_swimlane_in()
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.tile.get_swimlane_out()
    }
}

impl Ftile for FtileWithNotes {
    fn skin_param(&self) -> &Rc<SkinParam> {
        self.tile.skin_param()
    }

    fn get_in_link_rendering(&self) -> LinkRendering {
        self.tile.get_in_link_rendering()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        *self.cached_geometry.get_or_init(|| {
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

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        self.left
            .draw_u(&ug.apply(self.get_translate_for_left(string_bounder)));
        self.right
            .draw_u(&ug.apply(self.get_translate_for_right(string_bounder)));
        ug.apply(self.get_translate(string_bounder))
            .draw(&self.tile);
    }
}
