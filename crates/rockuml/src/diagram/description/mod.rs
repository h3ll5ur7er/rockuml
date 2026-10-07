//! Usecase, component, deployment and archimate diagrams (PlantUML's `descdiagram` package).

mod commands;

use std::rc::Rc;

use super::builder::CommandFactory;
use super::common_commands::add_common_commands1;
use super::cuca::CucaDiagram;
use super::cuca_commands::{self, note};
use super::diagram_type::DiagramType;
use super::titled::{PragmaKey, Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::command::factory::AbstractDiagram;
use crate::command::{Command, ParserPass};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::pattern::RegexTree;
use crate::style::SName;

/// Drawing description diagrams is not ported yet; it is reported as soon as the lines read as one.
const NOT_PORTED: NotYetPorted = NotYetPorted("usecase, component and deployment diagrams");

pub(super) struct DescriptionDiagram {
    source: Rc<UmlSource>,
    cuca: CucaDiagram,
}

/// Reads usecase, component and deployment diagrams (PlantUML's `DescriptionDiagramFactory`).
pub(super) struct DescriptionDiagramFactory;

impl CommandFactory for DescriptionDiagramFactory {
    type Diagram = DescriptionDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Description;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> DescriptionDiagram {
        let mut titled = Titled::new(SName::ComponentDiagram, "DESCRIPTION", source);
        titled.not_ported(NOT_PORTED);
        DescriptionDiagram {
            source: source.clone(),
            cuca: CucaDiagram::new(titled),
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<DescriptionDiagram>>> {
        let mut commands = vec![
            cuca_commands::footbox_ignored(),
            cuca_commands::rank_dir(),
            cuca_commands::newpage(),
        ];
        commands.extend(add_common_commands1());
        commands.extend([
            commands::link_element(),
            cuca_commands::hide_show2(),
            cuca_commands::remove_restore(),
            cuca_commands::package_with_usymbol(),
            cuca_commands::together(),
            cuca_commands::end_package(),
            note::note_multi_line(),
            note::note_on_link(ParserPass::One),
            note::note_on_link_multi_line(ParserPass::One),
            note::note_on_entity(code_for_description(), ParserPass::One),
            note::note(),
            cuca_commands::url(),
            commands::create_element_full(),
            cuca_commands::create_element_multilines_type0(),
            cuca_commands::create_element_multilines_type1(),
            note::note_on_entity_multi_line(code_for_description(), ParserPass::One, true),
            note::note_on_entity_multi_line(code_for_description(), ParserPass::One, false),
            note::note_multi_line(),
            cuca_commands::create_map(),
            cuca_commands::create_json(),
            cuca_commands::create_json_single_line(),
            commands::archimate(),
            commands::archimate_multilines(),
            commands::archimate_package(),
            commands::create_domain(),
        ]);
        commands
    }
}

/// How notes name the element they are on: by code, as `()`, `[]`, `( )` or `: :` shapes, or quoted.
fn code_for_description() -> RegexTree {
    RegexTree::named_or(
        "CODE",
        vec![
            RegexTree::leaf("[%pLN_.]+"),
            RegexTree::leaf(r"\(\)[%s]*[%pLN_.]+"),
            RegexTree::leaf(r"\(\)[%s]*[%g][^%g]+[%g]"),
            RegexTree::leaf(r"\[[^\]*]+[^\]]*\]"),
            RegexTree::leaf(r"\((?!\*\))[^\)]+\)"),
            RegexTree::leaf(":[^:]+:"),
            RegexTree::leaf("[%g][^%g]+[%g]"),
        ],
    )
}

impl AbstractDiagram for DescriptionDiagram {
    fn starting_pass(&mut self, _pass: ParserPass) {
        self.cuca.starting_pass();
    }

    fn check_final_error(&mut self) -> Option<String> {
        if self
            .cuca
            .titled
            .pragma
            .is_false(PragmaKey::UseIntermediatePackages)
        {
            self.cuca.pack_some_package();
        }
        self.cuca.apply_single_strategy();
        None
    }
}

impl TitledDiagram for DescriptionDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.cuca.titled
    }
}

impl cuca_commands::EntityDiagram for DescriptionDiagram {
    fn cuca(&mut self) -> &mut CucaDiagram {
        &mut self.cuca
    }
}

impl Diagram for DescriptionDiagram {
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
