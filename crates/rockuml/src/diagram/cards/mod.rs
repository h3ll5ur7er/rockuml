//! PlantUML's reference cards: diagrams of a single keyword line (or of none at all) that draw a built-in
//! card instead of a diagram of the user's.

mod colors;
mod license;
mod list_fonts;
mod skinparameter_list;
mod version;
mod welcome;

pub(super) use colors::PSystemColorsFactory;
pub(super) use license::PSystemLicenseFactory;
pub(super) use list_fonts::PSystemListFontsFactory;
pub(super) use skinparameter_list::PSystemSkinparameterListFactory;
pub(super) use version::PSystemVersionFactory;
pub(super) use welcome::{PSystemWelcome, PSystemWelcomeFactory};

use super::ug_diagram::{Card, UgDiagram};
use crate::klimt::TextBlock;
use crate::klimt::graphic_strings::{GraphicPosition, GraphicStrings};
use crate::klimt::image::PortableImage;
use crate::klimt::typeface::FontRegistry;

/// Lines of creole text with an optional image in a corner (PlantUML's `PlainStringsDiagram`).
struct PlainStringsDiagram {
    strings: Vec<String>,
    image: Option<(PortableImage, GraphicPosition)>,
}

impl PlainStringsDiagram {
    fn new(strings: Vec<String>) -> Self {
        Self {
            strings,
            image: None,
        }
    }

    fn with_plantuml_image(strings: Vec<String>) -> Self {
        Self {
            strings,
            image: Some((
                get_plantuml_image(),
                GraphicPosition::BackgroundCornerBottomRight,
            )),
        }
    }

    fn graphic_strings(&self) -> GraphicStrings {
        GraphicStrings::create_black_on_white_with(&self.strings, self.image.clone())
    }
}

impl Card for PlainStringsDiagram {
    fn get_text_block(&self, _fonts: &FontRegistry) -> Box<dyn TextBlock + '_> {
        Box::new(self.graphic_strings())
    }
}

/// PlantUML's logo (`PSystemVersion.getPlantumlImage`).
fn get_plantuml_image() -> PortableImage {
    PortableImage::from_png(
        crate::assets::get("images/plantuml-logo.png").expect("the logo is embedded"),
    )
    .expect("the logo is a PNG")
}
