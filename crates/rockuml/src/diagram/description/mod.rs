//! Usecase, component, deployment and archimate diagrams (PlantUML's `descdiagram` package).

mod commands;
#[cfg(test)]
mod tests;

use std::rc::Rc;

use super::builder::CommandFactory;
use super::common_commands::add_common_commands1;
use super::cuca::{CucaDiagram, EntityDiagram};
use super::cuca_commands::{self, note};
use super::diagram_type::DiagramType;
use super::titled::{PragmaKey, Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::abel::LeafType;
use crate::command::factory::AbstractDiagram;
use crate::command::{Command, ParserPass};
use crate::decoration::symbol::USymbols;
use crate::java;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::pattern::RegexTree;
use crate::style::SName;
use crate::text::without_quotes_or_brackets;

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
        let titled = Titled::new(SName::ComponentDiagram, "DESCRIPTION", source);
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
            note::note_on_entity(code_for_description, ParserPass::One),
            note::note(),
            cuca_commands::url(),
            commands::create_element_full(),
            cuca_commands::create_element_multilines_type0(),
            cuca_commands::create_element_multilines_type1(),
            note::note_on_entity_multi_line(code_for_description, ParserPass::One, true),
            note::note_on_entity_multi_line(code_for_description, ParserPass::One, false),
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

impl EntityDiagram for DescriptionDiagram {
    fn cuca(&mut self) -> &mut CucaDiagram {
        &mut self.cuca
    }

    /// Also `()x`, `:x:/` and `(x)/`, the notations of interfaces and business actors and use cases.
    fn clean_id(id: &str) -> &str {
        let id = id.strip_prefix("()").map_or(id, java::trim);
        if let Some(name) = id
            .strip_prefix(':')
            .and_then(|rest| rest.strip_suffix(":/"))
            .or_else(|| {
                id.strip_prefix('(')
                    .and_then(|rest| rest.strip_suffix(")/"))
            })
        {
            return name;
        }
        without_quotes_or_brackets(id)
    }
}

impl DescriptionDiagram {
    /// Whether some leaf is a use case or an actor of the diagram's actor style.
    fn is_usecase(&self) -> bool {
        let actor = self.cuca.skin().actor_style().to_u_symbol();
        self.cuca.leafs().into_iter().any(|leaf| {
            let leaf = self.cuca.entity(leaf);
            leaf.get_leaf_type() == Some(LeafType::Usecase) || leaf.get_usymbol() == Some(actor)
        })
    }
}

impl AbstractDiagram for DescriptionDiagram {
    fn starting_pass(&mut self, _pass: ParserPass) {
        self.cuca.starting_pass();
    }

    /// Names only links mention are actors in use case diagrams, and interfaces elsewhere.
    fn make_diagram_ready(&mut self) {
        let default_symbol = if self.is_usecase() {
            self.cuca.skin().actor_style().to_u_symbol()
        } else {
            USymbols::INTERFACE
        };
        for leaf in self.cuca.leafs() {
            let leaf = self.cuca.entity_mut(leaf);
            if leaf.get_leaf_type() == Some(LeafType::StillUnknown) {
                leaf.mute_to_type(LeafType::Description);
                leaf.usymbol = Some(default_symbol);
            }
        }
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

impl Diagram for DescriptionDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        let drawing = self.cuca.get_text_block(string_bounder.as_ref())?;
        Ok(self.cuca.titled.add_chrome(drawing, string_bounder))
    }

    fn export_settings(&self) -> ExportSettings {
        self.cuca
            .titled
            .export_settings(self.source.seed(), CucaDiagram::get_default_margins())
    }
}
