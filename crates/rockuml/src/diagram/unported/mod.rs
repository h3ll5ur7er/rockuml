//! `@startuml` diagram types rockuml cannot read yet. Their commands recognise the lines PlantUML's do, so
//! that a source is read as the diagram type it is in PlantUML, or fails with the same error.

mod activity;
mod activity3;
mod help;
mod list_sprite;
mod timing;

use std::rc::Rc;

use super::builder::CommandFactory;
use super::common_commands::{
    add_common_commands1, add_common_commands2, add_common_scale_commands,
};
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

/// Reads activity diagrams (PlantUML's `ActivityDiagramFactory3`).
pub(super) struct ActivityDiagramFactory3;

impl CommandFactory for ActivityDiagramFactory3 {
    type Diagram = UnportedDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Activity;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> UnportedDiagram {
        UnportedDiagram::new(source, SName::ActivityDiagram, "activity diagrams")
    }

    fn init_commands_list() -> Vec<Box<dyn Command<UnportedDiagram>>> {
        let mut commands = vec![cuca_commands::footbox_ignored()];
        commands.extend(add_common_commands1());
        commands.extend([
            activity3::swimlane(),
            activity3::swimlane2(),
            activity3::partition3(),
            activity3::close_group3(),
            activity3::close_group_legacy3(),
            activity3::arrow3(),
            activity3::arrow_long3(),
            activity3::repeat3(),
            activity3::activity3(),
            activity3::if4(),
            activity3::if2(),
            activity3::if2_multine(),
            activity3::if_legacy1(),
            activity3::else_if3(),
            activity3::else_if2(),
            activity3::else3(),
            activity3::else3_multine(),
            activity3::else_legacy1(),
            activity3::endif3(),
            activity3::switch(),
            activity3::case(),
            activity3::end_switch(),
            activity3::repeat_while3(),
            activity3::repeat_while3_multilines(),
            activity3::backward3(),
            activity3::backward_long3(),
            activity3::while3(),
            activity3::while_end3(),
            activity3::fork3(),
            activity3::fork_again3(),
            activity3::fork_end3(),
            activity3::split3(),
            activity3::split_again3(),
            activity3::split_end3(),
            activity3::start3(),
            activity3::stop3(),
            activity3::circle_spot3(),
            activity3::break_command(),
            activity3::end3(),
            activity3::kill3(),
            activity3::link3(),
            activity3::note3(),
            activity3::note_long3(),
            activity3::activity_long3(),
            activity3::activity_list(),
            activity3::label(),
            activity3::goto(),
            activity3::else_if2_multine(),
        ]);
        commands
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

/// Reads timing diagrams (PlantUML's `TimingDiagramFactory`).
pub(super) struct TimingDiagramFactory;

impl CommandFactory for TimingDiagramFactory {
    type Diagram = UnportedDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Timing;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> UnportedDiagram {
        UnportedDiagram::new(source, SName::TimingDiagram, "timing diagrams")
    }

    fn init_commands_list() -> Vec<Box<dyn Command<UnportedDiagram>>> {
        let mut commands = add_common_commands1();
        commands.extend([
            cuca_commands::footbox_ignored(),
            timing::robust_concise(),
            timing::clock(),
            timing::analog(),
            timing::binary(),
            timing::define_state_short(),
            timing::define_state_long(),
            timing::change_state_by_player_code(),
            timing::change_state_by_time(),
            timing::at_time(),
            timing::at_player(),
            timing::time_message(),
            timing::note(),
            timing::note_long(),
            timing::constraint(),
            timing::scale_pixel(),
            timing::hide_time_axis(),
            timing::highlight(),
            timing::mode_compact(),
            timing::ticks(),
            timing::pixel_height(),
            timing::use_date_format(),
        ]);
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
