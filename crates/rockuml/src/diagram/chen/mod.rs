//! Chen entity relationship diagrams, `@startchen` (PlantUML's `cheneer` package).

mod commands;

use std::rc::Rc;

use super::builder::CommandFactory;
use super::common_commands::add_common_commands1;
use super::cuca::CucaDiagram;
use super::cuca_commands;
use super::diagram_type::DiagramType;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::command::factory::AbstractDiagram;
use crate::command::{Command, ParserPass};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::style::SName;

/// Drawing Chen diagrams is not ported yet; it is reported as soon as the lines read as one.
const NOT_PORTED: NotYetPorted = NotYetPorted("Chen diagrams");

pub(super) struct ChenEerDiagram {
    source: Rc<UmlSource>,
    cuca: CucaDiagram,
}

/// Reads Chen diagrams (PlantUML's `ChenEerDiagramFactory`).
pub(super) struct ChenEerDiagramFactory;

impl CommandFactory for ChenEerDiagramFactory {
    type Diagram = ChenEerDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::ChenEer;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> ChenEerDiagram {
        let mut titled = Titled::new(SName::ChenEerDiagram, "CHEN_EER", source);
        titled.not_ported(NOT_PORTED);
        ChenEerDiagram {
            source: source.clone(),
            cuca: CucaDiagram::new(titled),
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<ChenEerDiagram>>> {
        let mut commands = add_common_commands1();
        commands.extend([
            cuca_commands::rank_dir(),
            commands::create_entity(),
            commands::create_attribute(),
            commands::associate(),
            commands::end_group(),
            commands::simple_subclass(),
            commands::multi_subclass(),
        ]);
        commands
    }
}

impl AbstractDiagram for ChenEerDiagram {
    fn starting_pass(&mut self, _pass: ParserPass) {
        self.cuca.starting_pass();
    }
}

impl TitledDiagram for ChenEerDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.cuca.titled
    }
}

impl Diagram for ChenEerDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        _string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        Err(NOT_PORTED)
    }

    fn export_settings(&self) -> ExportSettings {
        self.cuca
            .titled
            .export_settings(self.source.seed(), CucaDiagram::get_default_margins())
    }
}
