//! Diagrams that draw one text block of their own, without a skin: PlantUML's reference cards.

use std::rc::Rc;

use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::klimt::typeface::FontRegistry;

/// What a card draws, given the fonts rockuml draws with.
pub(super) trait Card {
    fn get_text_block(&self, fonts: &FontRegistry) -> Box<dyn TextBlock + '_>;
}

/// Most cards draw the same whatever the fonts.
impl<T: TextBlock> Card for T {
    fn get_text_block(&self, _fonts: &FontRegistry) -> Box<dyn TextBlock + '_> {
        Box::new(self)
    }
}

/// PlantUML's `UgDiagram`, for the diagrams that are no `TitledDiagram`.
pub(super) struct UgDiagram<C> {
    source: Rc<UmlSource>,
    card: C,
    margin: ClockwiseTopRightBottomLeft,
}

impl<C: Card + 'static> UgDiagram<C> {
    /// No margin, as PlantUML's `getDefaultMargins` has unless a diagram says otherwise.
    pub(super) fn create(source: &Rc<UmlSource>, card: C) -> Box<dyn Diagram> {
        Self::with_margin(source, card, ClockwiseTopRightBottomLeft::none())
    }

    pub(super) fn with_margin(
        source: &Rc<UmlSource>,
        card: C,
        margin: ClockwiseTopRightBottomLeft,
    ) -> Box<dyn Diagram> {
        Box::new(Self {
            source: source.clone(),
            card,
            margin,
        })
    }
}

impl<C: Card> Diagram for UgDiagram<C> {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        self.text_block_with_fonts(page, string_bounder, &FontRegistry::default())
    }

    fn text_block_with_fonts(
        &self,
        _page: usize,
        _string_bounder: &Rc<dyn StringBounder>,
        fonts: &FontRegistry,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        Ok(self.card.get_text_block(fonts))
    }

    fn export_settings(&self, _page: usize) -> ExportSettings {
        ExportSettings {
            margin: self.margin,
            ..ExportSettings::without_skin(self.source.seed())
        }
    }
}
