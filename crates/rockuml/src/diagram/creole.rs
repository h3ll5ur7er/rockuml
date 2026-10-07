use super::diagram_type::DiagramType;
use super::error::ErrorDiagram;
use super::source::UmlSource;
use super::{Diagram, ExportSettings, NotYetPorted};
use crate::creole::{CreoleParser, SheetBlock1, SheetBlock2};
use crate::klimt::font::StringBounder;
use crate::klimt::font::{FontConfiguration, UFont};
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::preproc::start_utils;
use crate::text::StringLocated;
use std::rc::Rc;

/// `@startcreole`: the lines are creole markup, shown as they are.
pub(super) struct CreoleDiagram {
    source: UmlSource,
    lines: Vec<String>,
}

impl CreoleDiagram {
    /// The diagram, or an error image when there is nothing between the start and end lines.
    pub(super) fn create(source: UmlSource) -> Box<dyn Diagram> {
        let source = source.without_initial_noise();
        let mut lines = Vec::new();
        let content = source
            .lines()
            .iter()
            .skip(1)
            .map(StringLocated::text)
            .skip_while(|line| crate::java::trim(line).is_empty());
        for line in content {
            if start_utils::is_end_directive(line) {
                if source.lines().len() == 2 {
                    let trace = source.lines().to_vec();
                    return Box::new(ErrorDiagram::new(
                        source,
                        trace,
                        "Empty description",
                        Some(DiagramType::Creole),
                    ));
                }
                break;
            }
            lines.push(line.to_owned());
        }
        Box::new(Self { source, lines })
    }
}

impl Diagram for CreoleDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        _string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        let font = FontConfiguration::black_blue_true(UFont::serif(14));
        let sheet = CreoleParser::new(font, HorizontalAlignment::Left).create_sheet(&self.lines);
        // Unlike PlantUML, whose sheet has no stencil here and so cannot draw its separators.
        Ok(Box::new(SheetBlock2::new(SheetBlock1::new(
            sheet,
            ClockwiseTopRightBottomLeft::none(),
        ))))
    }

    fn export_settings(&self) -> ExportSettings {
        ExportSettings::without_skin(self.source.seed())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::diagram::{ImageFormat, export};
    use crate::fonts::FontRegistry;
    use crate::host::IsolatedHost;
    use crate::text::{LineLocation, StringLocated};

    #[test]
    fn separators_span_the_sheet() {
        let lines = ["@startcreole", "Some text", "----", "More", "@endcreole"];
        let location = LineLocation::new("test", None);
        let source = UmlSource::new(
            lines
                .iter()
                .map(|line| StringLocated::new(*line, location.clone()))
                .collect(),
            lines.iter().map(|line| (*line).to_owned()).collect(),
        );
        let diagram = CreoleDiagram::create(source);
        let debug = export(
            diagram.as_ref(),
            ImageFormat::Debug,
            &Arc::new(FontRegistry::default()),
            &IsolatedHost,
        )
        .unwrap();
        let debug = String::from_utf8(debug).unwrap();
        assert!(
            debug.contains(
                "LINE:
  pt1: [ 0.0000 ; "
            ),
            "{debug}"
        );
    }
}
