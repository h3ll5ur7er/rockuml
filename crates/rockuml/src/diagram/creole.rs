use super::source::UmlSource;
use super::{Diagram, ExportSettings, NotYetPorted};
use crate::creole::{CreoleParser, SheetBlock1};
use crate::klimt::font::{FontConfiguration, UFont};
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::preproc::start_utils;
use crate::text::StringLocated;

/// `@startcreole`: the lines are creole markup, shown as they are.
pub struct CreoleDiagram {
    source: UmlSource,
    lines: Vec<String>,
}

impl CreoleDiagram {
    pub fn create(source: UmlSource) -> Result<Self, NotYetPorted> {
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
                    return Err(NotYetPorted("error diagram for an empty description"));
                }
                break;
            }
            lines.push(line.to_owned());
        }
        Ok(Self { source, lines })
    }
}

impl Diagram for CreoleDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(&self) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        let font = FontConfiguration::black_blue_true(UFont::serif(14));
        let sheet = CreoleParser::new(font, HorizontalAlignment::Left).create_sheet(&self.lines);
        Ok(Box::new(SheetBlock1::new(
            sheet,
            ClockwiseTopRightBottomLeft::none(),
        )))
    }

    fn export_settings(&self) -> ExportSettings {
        ExportSettings::without_skin(self.source.seed())
    }
}
