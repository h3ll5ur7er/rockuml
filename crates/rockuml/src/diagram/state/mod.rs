//! State diagrams (PlantUML's `statediagram` package). Their lines are read three times: links may name
//! states declared further down, and notes may name states links created.

mod commands;

use std::rc::Rc;

use super::builder::CommandFactory;
use super::common_commands::add_common_commands1;
use super::cuca::CucaDiagram;
use super::cuca_commands::{self, note};
use super::diagram_type::DiagramType;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::abel::{EntityId, GroupType};
use crate::command::factory::AbstractDiagram;
use crate::command::{Command, ParserPass};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::pattern::RegexTree;
use crate::style::SName;

/// Drawing state diagrams is not ported yet; it is reported as soon as the lines read as one.
const NOT_PORTED: NotYetPorted = NotYetPorted("state diagrams");

pub(super) struct StateDiagram {
    source: Rc<UmlSource>,
    cuca: CucaDiagram,
}

/// Reads state diagrams (PlantUML's `StateDiagramFactory`).
pub(super) struct StateDiagramFactory;

impl CommandFactory for StateDiagramFactory {
    type Diagram = StateDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::State;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> StateDiagram {
        let mut titled = Titled::new(SName::StateDiagram, "STATE", source);
        titled.not_ported(NOT_PORTED);
        let mut cuca = CucaDiagram::new(titled);
        cuca.set_namespace_separator(Some("."));
        StateDiagram {
            source: source.clone(),
            cuca,
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<StateDiagram>>> {
        let mut commands = vec![
            cuca_commands::footbox_ignored(),
            cuca_commands::rank_dir(),
            cuca_commands::remove_restore(),
            commands::create_state(),
            commands::link_state(),
            commands::link_state_reverse(),
            commands::create_package_state(),
            commands::create_package2(),
            commands::end_state(),
            commands::add_field(),
            commands::concurrent_state(),
            note::note_on_entity_multi_line(code_for_state(), ParserPass::Three, true),
            note::note_on_entity_multi_line(code_for_state(), ParserPass::Three, false),
            note::note_on_entity(code_for_state(), ParserPass::Three),
            note::note_on_link(ParserPass::Two),
            note::note_on_link_multi_line(ParserPass::Two),
            cuca_commands::url(),
            note::note(),
            note::note_multi_line(),
            cuca_commands::create_map(),
            cuca_commands::create_json(),
            cuca_commands::create_json_single_line(),
        ];
        commands.extend(add_common_commands1());
        commands.push(cuca_commands::hide_show2());
        commands
    }
}

/// How notes name the state they are on.
fn code_for_state() -> RegexTree {
    RegexTree::named_or(
        "CODE",
        vec![
            RegexTree::leaf("[%pLN_.]+"),
            RegexTree::leaf("[%g][^%g]+[%g]"),
        ],
    )
}

impl AbstractDiagram for StateDiagram {
    fn required_pass(&self) -> &'static [ParserPass] {
        &[ParserPass::One, ParserPass::Two, ParserPass::Three]
    }

    fn starting_pass(&mut self, _pass: ParserPass) {
        self.cuca.starting_pass();
    }

    /// A link may not leave the concurrent region its state is in.
    fn check_final_error(&mut self) -> Option<String> {
        let cuca = &self.cuca;
        cuca.get_links()
            .find(|link| {
                concurrent_region(cuca, link.get_entity1())
                    != concurrent_region(cuca, link.get_entity2())
            })
            .map(|link| {
                format!(
                    "State within concurrent state cannot be linked out of this concurrent state (between {} and {})",
                    cuca.entity(link.get_entity1()).get_name(cuca),
                    cuca.entity(link.get_entity2()).get_name(cuca)
                )
            })
    }
}

/// The innermost concurrent region around `entity` (`getGroupParentIfItIsConcurrentState`).
fn concurrent_region(cuca: &CucaDiagram, entity: EntityId) -> Option<EntityId> {
    let mut parent = cuca.entity(entity).get_parent_container(cuca);
    while let Some(group) = parent {
        if cuca.entity(group).get_group_type() == GroupType::ConcurrentState {
            return Some(group);
        }
        parent = cuca.entity(group).get_parent_container(cuca);
    }
    None
}

impl TitledDiagram for StateDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.cuca.titled
    }
}

impl cuca_commands::EntityDiagram for StateDiagram {
    fn cuca(&mut self) -> &mut CucaDiagram {
        &mut self.cuca
    }
}

impl Diagram for StateDiagram {
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
