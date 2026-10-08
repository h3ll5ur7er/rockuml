//! PlantUML's `FtileFactoryDelegatorAddNote`: notes beside tiles, or alone.

use std::rc::Rc;

use super::{FtileNoteAlone, FtileWithNoteOpale};
use crate::diagram::activity3::{NoteType, PositionedNote, SwimlaneId};
use crate::ftile::{Ftile, FtileFactory, FtileFactoryDelegator};
use crate::klimt::VerticalAlignment;

pub(crate) struct FtileFactoryDelegatorAddNote {
    factory: Box<dyn FtileFactory>,
}

impl FtileFactoryDelegatorAddNote {
    pub(crate) fn new(factory: Box<dyn FtileFactory>) -> Self {
        Self { factory }
    }
}

impl FtileFactoryDelegator for FtileFactoryDelegatorAddNote {
    fn get_factory(&self) -> &dyn FtileFactory {
        self.factory.as_ref()
    }

    /// Without a tile, only the first note shows.
    fn add_note(
        &self,
        ftile: Option<Rc<dyn Ftile>>,
        swimlane: Option<SwimlaneId>,
        notes: &[PositionedNote],
        vertical_alignment: VerticalAlignment,
    ) -> Rc<dyn Ftile> {
        let skin_param = FtileFactoryDelegator::skin_param(self).clone();
        let Some(ftile) = ftile else {
            let note = notes
                .first()
                .expect("a tile gets notes only when it has some");
            return Rc::new(FtileNoteAlone::new(
                &note.display,
                skin_param,
                &note.colors,
                note.type_ == NoteType::Note,
                swimlane,
            ));
        };
        FtileWithNoteOpale::create(ftile, skin_param, notes, true, vertical_alignment)
    }
}
