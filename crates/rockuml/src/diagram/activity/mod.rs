//! Legacy activity diagrams, `(*) --> "First"` (PlantUML's `activitydiagram` package): actions, branches and
//! synchronization bars linked one after the other, laid out like the other entity diagrams.

mod commands;
#[cfg(test)]
mod graph_tests;
#[cfg(test)]
mod tests;

use std::rc::Rc;

use super::builder::CommandFactory;
use super::cuca::{CucaDiagram, EntityDiagram};
use super::diagram_type::DiagramType;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::abel::{EntityId, GroupType, LeafType};
use crate::command::factory::AbstractDiagram;
use crate::command::{Command, CommandError};
use crate::creole::Display;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::plasma::QuarkId;
use crate::style::SName;
use crate::text::LineLocation;

pub(super) struct ActivityDiagram {
    source: Rc<UmlSource>,
    cuca: CucaDiagram,
    /// Where a link without source starts: the latest action, branch or bar named.
    last_entity_consulted: Option<EntityId>,
    /// The branch of each `if` the commands are inside, innermost last (PlantUML's chain of
    /// `ConditionalContext`).
    conditional_contexts: Vec<EntityId>,
}

/// Reads legacy activity diagrams (PlantUML's `ActivityDiagramFactory`).
pub(super) struct ActivityDiagramFactory;

impl CommandFactory for ActivityDiagramFactory {
    type Diagram = ActivityDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Activity;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> ActivityDiagram {
        let titled = Titled::new(SName::ActivityDiagram, "ACTIVITY", source);
        let mut cuca = CucaDiagram::new(titled);
        cuca.set_namespace_separator(None);
        ActivityDiagram {
            source: source.clone(),
            cuca,
            last_entity_consulted: None,
            conditional_contexts: Vec::new(),
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<ActivityDiagram>>> {
        commands::init_commands_list()
    }
}

impl ActivityDiagram {
    /// A leaf for `quark`, which becomes the latest entity consulted unless it is a note (`updateLasts`).
    fn really_create_leaf(
        &mut self,
        location: &LineLocation,
        quark: QuarkId,
        display: Display,
        leaf_type: LeafType,
    ) -> Result<EntityId, CommandError> {
        let result = self
            .cuca
            .really_create_leaf(Some(location), quark, display, leaf_type)?;
        if leaf_type != LeafType::Note {
            self.last_entity_consulted = Some(result);
        }
        Ok(result)
    }

    /// The entity `quark` holds, or a new leaf of `leaf_type` showing `display`.
    fn leaf_named(
        &mut self,
        location: &LineLocation,
        quark: QuarkId,
        display: Display,
        leaf_type: LeafType,
    ) -> Result<EntityId, CommandError> {
        match self.cuca.quark(quark).get_data() {
            Some(existing) => Ok(existing),
            None => self.really_create_leaf(location, quark, display, leaf_type),
        }
    }

    /// `(*)` where a link starts.
    fn get_start(&mut self, location: &LineLocation) -> Result<EntityId, CommandError> {
        let quark = self.cuca.quark_in_context(true, "start")?;
        self.leaf_named(
            location,
            quark,
            Display::with_newlines("start"),
            LeafType::CircleStart,
        )
    }

    /// `(*)` where a link ends, or `(*supp_id)` for one of several ends.
    fn get_end(
        &mut self,
        location: &LineLocation,
        supp_id: Option<&str>,
    ) -> Result<EntityId, CommandError> {
        let name = supp_id.map_or_else(|| "end".to_owned(), |supp_id| format!("end${supp_id}"));
        let quark = self.cuca.quark_in_context(true, &name)?;
        self.leaf_named(
            location,
            quark,
            Display::with_newlines("end"),
            LeafType::CircleEnd,
        )
    }

    /// A new branch, `#<n>` unless the `if` names it, which the commands are now inside.
    fn start_if(
        &mut self,
        location: &LineLocation,
        optional_code: Option<&str>,
    ) -> Result<(), CommandError> {
        let id_short =
            optional_code.map_or_else(|| self.cuca.get_unique_sequence("#"), str::to_owned);
        let quark = self
            .cuca
            .quark_in_context(true, Self::clean_id(&id_short))?;
        let branch = self.really_create_leaf(
            location,
            quark,
            Display::with_newlines(""),
            LeafType::Branch,
        )?;
        self.conditional_contexts.push(branch);
        Ok(())
    }

    /// `-->{`: a group of actions linked as one, which the commands are now inside.
    fn create_inner_activity(&mut self, location: &LineLocation) -> Result<EntityId, CommandError> {
        let id_short = self.cuca.get_unique_sequence("##");
        let quark = self.cuca.quark_in_context(true, &id_short)?;
        let display = Display::with_newlines(self.cuca.quark(quark).get_name());
        self.cuca
            .goto_group(Some(location), quark, display, GroupType::InnerActivity);
        self.last_entity_consulted = None;
        self.cuca.get_current_group()
    }

    /// The branch of the innermost `if`.
    fn current_branch(&self) -> Option<EntityId> {
        self.conditional_contexts.last().copied()
    }
}

impl AbstractDiagram for ActivityDiagram {}

impl EntityDiagram for ActivityDiagram {
    fn cuca(&mut self) -> &mut CucaDiagram {
        &mut self.cuca
    }
}

impl TitledDiagram for ActivityDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.cuca.titled
    }

    fn entity_diagram(&mut self) -> Option<&mut CucaDiagram> {
        Some(&mut self.cuca)
    }
}

impl Diagram for ActivityDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        let cuca = &self.cuca;
        if cuca
            .groups()
            .into_iter()
            .any(|group| cuca.entity(group).get_group_type() == GroupType::InnerActivity)
        {
            return Err(NotYetPorted("inner activities"));
        }
        let drawing = self.cuca.get_text_block(string_bounder.as_ref())?;
        Ok(self.cuca.titled.add_chrome(drawing, string_bounder))
    }

    fn export_settings(&self, _page: usize) -> ExportSettings {
        self.cuca
            .titled
            .export_settings(self.source.seed(), CucaDiagram::get_default_margins())
    }
}
