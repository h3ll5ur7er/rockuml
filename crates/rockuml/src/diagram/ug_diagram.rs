//! Diagrams that draw one text block of their own, without a skin: PlantUML's reference cards.

use std::rc::Rc;

use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::ClockwiseTopRightBottomLeft;

/// PlantUML's `UgDiagram`, for the diagrams that are no `TitledDiagram`.
pub(super) struct UgDiagram<T> {
    source: Rc<UmlSource>,
    drawing: T,
    margin: ClockwiseTopRightBottomLeft,
}

impl<T: TextBlock + 'static> UgDiagram<T> {
    /// No margin, as PlantUML's `getDefaultMargins` has unless a diagram says otherwise.
    pub(super) fn create(source: &Rc<UmlSource>, drawing: T) -> Box<dyn Diagram> {
        Self::with_margin(source, drawing, ClockwiseTopRightBottomLeft::none())
    }

    pub(super) fn with_margin(
        source: &Rc<UmlSource>,
        drawing: T,
        margin: ClockwiseTopRightBottomLeft,
    ) -> Box<dyn Diagram> {
        Box::new(Self {
            source: source.clone(),
            drawing,
            margin,
        })
    }
}

impl<T: TextBlock> Diagram for UgDiagram<T> {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        _string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        Ok(Box::new(&self.drawing))
    }

    fn export_settings(&self, _page: usize) -> ExportSettings {
        ExportSettings {
            margin: self.margin,
            ..ExportSettings::without_skin(self.source.seed())
        }
    }
}
