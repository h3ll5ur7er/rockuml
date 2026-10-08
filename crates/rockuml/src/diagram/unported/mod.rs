//! `@startuml` diagram types rockuml cannot read yet. Their commands recognise the lines PlantUML's do, so
//! that a source is read as the diagram type it is in PlantUML, or fails with the same error.

mod activity;
mod help;
mod list_sprite;

use std::rc::Rc;

use super::builder::CommandFactory;
use super::common_commands::{
    add_common_commands1, add_common_commands2, add_common_scale_commands,
};
use super::diagram_type::DiagramType;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::command::Command;
use crate::command::factory::AbstractDiagram;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::style::SName;

/// The diagram the lines of a type rockuml cannot read yet make.
pub(super) struct UnportedDiagram {
    source: Rc<UmlSource>,
    titled: Titled,
    what: NotYetPorted,
}

impl UnportedDiagram {
    fn new(source: &Rc<UmlSource>, style: SName, what: &'static str) -> Self {
        let mut titled = Titled::new(style, "UNPORTED", source);
        titled.not_ported(NotYetPorted(what));
        Self {
            source: source.clone(),
            titled,
            what: NotYetPorted(what),
        }
    }
}

impl AbstractDiagram for UnportedDiagram {}

impl TitledDiagram for UnportedDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
    }
}

impl Diagram for UnportedDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        _string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        Err(self.what)
    }

    fn export_settings(&self) -> ExportSettings {
        self.titled
            .export_settings(self.source.seed(), ClockwiseTopRightBottomLeft::none())
    }
}

/// Reads legacy activity diagrams (PlantUML's `ActivityDiagramFactory`).
pub(super) struct ActivityDiagramFactory;

impl CommandFactory for ActivityDiagramFactory {
    type Diagram = UnportedDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Activity;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> UnportedDiagram {
        UnportedDiagram::new(source, SName::ActivityDiagram, "legacy activity diagrams")
    }

    fn init_commands_list() -> Vec<Box<dyn Command<UnportedDiagram>>> {
        activity::init_commands_list()
    }
}

/// Reads sprite lists (PlantUML's `ListSpriteDiagramFactory`).
pub(super) struct ListSpriteDiagramFactory;

impl CommandFactory for ListSpriteDiagramFactory {
    type Diagram = UnportedDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Sprites;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> UnportedDiagram {
        UnportedDiagram::new(source, SName::ActivityDiagram, "sprite lists")
    }

    fn init_commands_list() -> Vec<Box<dyn Command<UnportedDiagram>>> {
        let mut commands = add_common_commands1();
        commands.extend(add_common_commands2());
        commands.extend(add_common_scale_commands());
        commands.push(list_sprite::list_sprite());
        commands
    }
}

/// Reads help requests like `help keywords` (PlantUML's `HelpFactory`).
pub(super) struct HelpFactory;

impl CommandFactory for HelpFactory {
    type Diagram = UnportedDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Help;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> UnportedDiagram {
        UnportedDiagram::new(source, SName::ActivityDiagram, "help diagrams")
    }

    fn init_commands_list() -> Vec<Box<dyn Command<UnportedDiagram>>> {
        vec![
            help::help_color(),
            help::help_font(),
            help::help_keyword(),
            help::help_type(),
            help::help_theme(),
        ]
    }
}
