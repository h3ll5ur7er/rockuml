//! A note on a link, drawn in the link's label (PlantUML's `EntityImageNoteLink`).

use crate::color::Colors;
use crate::creole::Display;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;
use crate::skin::component::{Area, Component, Context2D};
use crate::skin::rose::{NoteShape, create_component_note};
use crate::style::{SName, StyleBuilder, StyleSignature};

pub(crate) struct EntityImageNoteLink {
    comp: Box<dyn Component>,
}

impl EntityImageNoteLink {
    /// Drawn like a sequence diagram's note, whatever the diagram (`ComponentType.NOTE`).
    pub(crate) fn new(
        note: &Display,
        colors: &Colors,
        skin_param: &SkinParam,
        style_builder: &StyleBuilder,
    ) -> Self {
        let style = style_builder
            .merged_style(&StyleSignature::of(&[
                SName::Root,
                SName::Element,
                SName::SequenceDiagram,
                SName::Note,
            ]))
            .expect("the skin styles notes");
        Self {
            comp: create_component_note(&style, NoteShape::Folded, skin_param, note, colors, false),
        }
    }
}

impl TextBlock for EntityImageNoteLink {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.comp.preferred_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let dimension = self.calculate_dimension(ug.string_bounder());
        self.comp.draw_u(
            ug,
            &Area::new(dimension.width, dimension.height),
            Context2D {
                is_background: false,
            },
        );
    }
}
