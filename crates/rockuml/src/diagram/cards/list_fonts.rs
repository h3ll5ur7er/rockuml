//! The font list, `listfonts`: every font family rockuml can draw with, each with a sample.

use std::rc::Rc;

use super::{Card, PlainStringsDiagram, UgDiagram};
use crate::diagram::builder::PSystemSingleLineFactory;
use crate::diagram::{Diagram, NotYetPorted, UmlSource};
use crate::klimt::TextBlock;
use crate::klimt::typeface::FontRegistry;

/// `listfonts`, optionally followed by the sample text (PlantUML's `PSystemListFontsFactory`).
pub(in crate::diagram) struct PSystemListFontsFactory;

impl PSystemSingleLineFactory for PSystemListFontsFactory {
    fn execute_line(
        source: &Rc<UmlSource>,
        line: &str,
    ) -> Option<Result<Box<dyn Diagram>, NotYetPorted>> {
        let line_lower = line.to_lowercase();
        let listed = ["listfont", "listfonts"].contains(&line_lower.as_str())
            || line_lower.starts_with("listfont ")
            || line_lower.starts_with("listfonts ");
        if !listed {
            return None;
        }
        let text = match line.find(' ') {
            Some(index) => crate::java::trim(&line[index..]).to_owned(),
            None => "This is a test".to_owned(),
        };
        Some(Ok(UgDiagram::create(source, PSystemListFonts { text })))
    }
}

/// PlantUML's `PSystemListFonts`, which lists the fonts Java finds on the machine; rockuml lists those it
/// embeds and those registered with it.
struct PSystemListFonts {
    text: String,
}

impl PSystemListFonts {
    fn strings(&self, fonts: &FontRegistry) -> Vec<String> {
        let mut strings = vec!["   <b><size:16>Fonts available:".to_owned(), " ".to_owned()];
        for n in fonts.available_font_family_names() {
            strings.push(format!("{n} : <font:{n}>{}", self.text));
        }
        strings
    }
}

impl Card for PSystemListFonts {
    fn get_text_block(&self, fonts: &FontRegistry) -> Box<dyn TextBlock + '_> {
        Box::new(PlainStringsDiagram::new(self.strings(fonts)).graphic_strings())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_font_shows_the_sample_in_itself() {
        let list = PSystemListFonts {
            text: "Hello".to_owned(),
        };
        let strings = list.strings(&FontRegistry::default());
        assert_eq!(strings[0], "   <b><size:16>Fonts available:");
        assert!(strings.contains(&"Liberation Sans : <font:Liberation Sans>Hello".to_owned()));
        assert!(strings.contains(&"SansSerif : <font:SansSerif>Hello".to_owned()));
    }
}
