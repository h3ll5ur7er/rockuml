//! `@startdef`: a definition that `!includedef` reads, drawn on its own as its start line alone (PlantUML's
//! `PSystemDefinitionFactory` and `PSystemDefinition`).

use std::rc::Rc;

use super::ug_diagram::UgDiagram;
use super::{Diagram, UmlSource};
use crate::color::HColor;
use crate::creole::{CreoleMode, Display, SheetBlock2};
use crate::klimt::HorizontalAlignment;
use crate::klimt::font::{FontConfiguration, UFont, UFontFace};
use crate::klimt::sprite::SpriteContainerEmpty;
use crate::klimt::ugraphic::UStroke;
use crate::text::StringLocated;

/// PlantUML's `PSystemDefinitionFactory`. Every line is accepted, and an empty definition is no error.
pub(super) fn create(source: UmlSource) -> Box<dyn Diagram> {
    let source = Rc::new(source.without_initial_noise());
    let start_line = source.lines().first().map_or("", StringLocated::text);
    UgDiagram::create(&source, get_text_block(start_line))
}

/// The start line in black `SansSerif` 14, its links neither coloured nor underlined.
fn get_text_block(start_line: &str) -> SheetBlock2 {
    let font = FontConfiguration::new(
        UFont::new("SansSerif", UFontFace::NORMAL, 14),
        HColor::BLACK,
        8,
    )
    .with_hyperlink_style(HColor::BLACK, UStroke::with_thickness(0.0));
    Display::with_newlines(start_line).create0(
        &font,
        HorizontalAlignment::Left,
        &SpriteContainerEmpty,
        0.0,
        CreoleMode::Full,
    )
}
