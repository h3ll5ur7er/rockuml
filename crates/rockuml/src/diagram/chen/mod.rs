//! Chen entity relationship diagrams, `@startchen` (PlantUML's `cheneer` package).

mod commands;

use std::rc::Rc;

use super::builder::CommandFactory;
use super::common_commands::add_common_commands1;
use super::cuca_commands;
use super::diagram_type::DiagramType;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::command::Command;
use crate::command::factory::AbstractDiagram;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::style::SName;

pub(super) struct ChenEerDiagram {
    source: Rc<UmlSource>,
    titled: Titled,
}

/// Reads Chen diagrams (PlantUML's `ChenEerDiagramFactory`).
pub(super) struct ChenEerDiagramFactory;

impl CommandFactory for ChenEerDiagramFactory {
    type Diagram = ChenEerDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::ChenEer;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> ChenEerDiagram {
        ChenEerDiagram {
            titled: Titled::new(SName::ChenEerDiagram, "CHEN_EER", source),
            source: source.clone(),
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

impl AbstractDiagram for ChenEerDiagram {}

impl TitledDiagram for ChenEerDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
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
        Err(NotYetPorted("Chen diagrams"))
    }

    fn export_settings(&self) -> ExportSettings {
        self.titled.export_settings(
            self.source.seed(),
            ClockwiseTopRightBottomLeft::top_right_bottom_left(0.0, 5.0, 5.0, 0.0),
        )
    }
}
