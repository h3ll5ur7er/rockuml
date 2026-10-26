//! PlantUML's `GraphicStrings`: lines of creole text on a background, inside a margin, as PlantUML's own
//! cards draw their text.

use super::font::{FontConfiguration, StringBounder, UFont, UFontFace};
use super::geom::XDimension2D;
use super::sprite::SpriteContainerEmpty;
use super::ugraphic::UGraphic;
use super::{HorizontalAlignment, TextBlock};
use crate::color::HColor;
use crate::creole::{CreoleMode, Display, SheetBlock2};

const MARGIN: f64 = 5.0;

pub(crate) struct GraphicStrings {
    text: SheetBlock2,
    background: HColor,
}

impl GraphicStrings {
    /// Black `SansSerif` 12 on white (`createBlackOnWhite`).
    pub(crate) fn create_black_on_white(
        lines: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let font = FontConfiguration::new(
            UFont::new("SansSerif", UFontFace::NORMAL, 12),
            HColor::BLACK,
            8,
        );
        Self {
            text: Display::create(lines).create0(
                &font,
                HorizontalAlignment::Left,
                &SpriteContainerEmpty,
                0.0,
                CreoleMode::Full,
            ),
            background: HColor::WHITE,
        }
    }
}

impl TextBlock for GraphicStrings {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.text
            .calculate_dimension(string_bounder)
            .delta(2.0 * MARGIN, 2.0 * MARGIN)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.text.draw_u(&ug.translated(MARGIN, MARGIN));
    }

    fn backcolor(&self) -> Option<HColor> {
        Some(self.background.clone())
    }
}
