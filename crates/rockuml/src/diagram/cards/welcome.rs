//! The welcome screen of a source with nothing between its start and end lines.

use std::rc::Rc;

use super::{Card, UgDiagram, get_plantuml_image};
use crate::diagram::UmlSource;
use crate::diagram::builder::Outcome;
use crate::klimt::TextBlock;
use crate::klimt::graphic_strings::{GraphicPosition, GraphicStrings};
use crate::klimt::typeface::FontRegistry;

/// `@startuml` and `@enduml` alone (PlantUML's `PSystemWelcomeFactory`).
pub(in crate::diagram) struct PSystemWelcomeFactory;

impl PSystemWelcomeFactory {
    pub(in crate::diagram) fn create_system(source: &Rc<UmlSource>) -> Outcome {
        if source.lines().len() == 2 {
            Outcome::Diagram(Ok(UgDiagram::boxed(
                source,
                PSystemWelcome::new(GraphicPosition::BackgroundCornerBottomRight),
            )))
        } else {
            Outcome::Nothing
        }
    }
}

/// How to start, with PlantUML's logo in a corner; error images of very short sources show it too
/// (PlantUML's `PSystemWelcome`).
pub(in crate::diagram) struct PSystemWelcome {
    position: GraphicPosition,
}

impl PSystemWelcome {
    pub(in crate::diagram) fn new(position: GraphicPosition) -> Self {
        Self { position }
    }

    pub(in crate::diagram) fn text_block(&self) -> GraphicStrings {
        let mut strings = vec![
            "<b>Welcome to PlantUML!",
            " ",
            "You can start with a simple UML Diagram like:",
            " ",
            "\"\"Bob->Alice: Hello\"\"",
            " ",
            "Or",
            " ",
            "\"\"class Example\"\"",
            " ",
            "You will find more information about PlantUML syntax on <u>https://plantuml.com</u>",
            " ",
            "(Details by typing \"\"license\"\" keyword)",
            " ",
        ];
        // Room for the logo below the text.
        if self.position == GraphicPosition::BackgroundCornerBottomRight {
            strings.extend([" "; 4]);
        }
        GraphicStrings::create_black_on_white_with(
            &strings,
            Some((get_plantuml_image(), self.position)),
        )
    }
}

impl Card for PSystemWelcome {
    fn get_text_block(&self, _fonts: &FontRegistry) -> Box<dyn TextBlock + '_> {
        Box::new(self.text_block())
    }
}
